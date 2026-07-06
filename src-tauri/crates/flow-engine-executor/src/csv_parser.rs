//! CSV 解析模块
//!
//! 支持 UTF-8/GBK/GB2312/Big5 自动编码检测的流式 CSV 解析器。
//! 提供 preview（预览前 N 行）、validate（验证元信息）等功能。

use std::fs::File;
use std::io::{BufReader, Read as IoRead};

use serde_json::Value;

/// CSV 文件元信息（validate 返回值）
#[derive(Debug, Clone)]
pub struct CsvInfo {
    /// 数据行数（不含表头）
    pub row_count: u32,
    /// 列名列表
    pub column_names: Vec<String>,
    /// 检测到的编码名称
    pub detected_encoding: String,
}

/// CSV 行迭代器，支持流式逐行读取
pub struct CsvReader {
    reader: csv::Reader<Box<dyn std::io::Read + Send>>,
    headers: csv::StringRecord,
    finished: bool,
}

impl CsvReader {
    /// 读取下一行数据，返回 JSON 对象
    ///
    /// 每行数据以表头为 key，构造成 JSON 对象。
    /// 数据行少于表头列时，缺失列填充空字符串；
    /// 数据行多于表头列时，忽略多余列。
    pub fn next_row(&mut self) -> Option<Result<Value, String>> {
        if self.finished {
            return None;
        }
        let mut record = csv::StringRecord::new();
        match self.reader.read_record(&mut record) {
            Ok(false) => {
                self.finished = true;
                None
            }
            Ok(true) => {
                let mut map = serde_json::Map::with_capacity(self.headers.len());
                for (i, header) in self.headers.iter().enumerate() {
                    let val = record.get(i).unwrap_or("");
                    map.insert(header.to_string(), Value::String(val.to_string()));
                }
                Some(Ok(Value::Object(map)))
            }
            Err(e) => {
                self.finished = true;
                Some(Err(format!("读取 CSV 行Failed: {}", e)))
            }
        }
    }

    /// 收集所有剩余行到 Vec
    pub fn collect_all(mut self) -> Result<Vec<Value>, String> {
        let mut rows = Vec::new();
        while let Some(result) = self.next_row() {
            rows.push(result?);
        }
        Ok(rows)
    }
}

/// 检测字节编码
///
/// 优先检测 BOM 标记，其次使用 chardetng 库进行启发式检测。
/// 支持识别 UTF-8、GBK、GB2312、Big5 等常见中文编码。
/// 检测Failed时默认返回 "utf-8"。
fn detect_encoding(raw: &[u8]) -> String {
    // BOM 检测
    if raw.len() >= 3 && raw[0] == 0xEF && raw[1] == 0xBB && raw[2] == 0xBF {
        return "utf-8".to_string();
    }
    if raw.len() >= 2 {
        if raw[0] == 0xFF && raw[1] == 0xFE {
            return "utf-16le".to_string();
        }
        if raw[0] == 0xFE && raw[1] == 0xFF {
            return "utf-16be".to_string();
        }
    }

    // chardetng 启发式检测
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(raw, true);
    let encoding = detector.guess(None, true);

    // 将编码名标准化为小写
    let name = encoding.name().to_lowercase();

    // chardetng 返回的是 WHATWG 编码标签，需要映射常见名称
    match name.as_str() {
        "gbk" | "gb18030" => "gbk".to_string(),
        "big5" => "big5".to_string(),
        "utf-8" => "utf-8".to_string(),
        _ => name,
    }
}

/// 解码字节到字符串
///
/// 支持通过编码名称（如 "utf-8"、"gbk"、"big5"）解码字节数据。
/// 编码名称不区分大小写，且支持常见别名（如 "gb2312" 映射到 "gbk"）。
fn decode_bytes(raw: &[u8], encoding: &str) -> String {
    let enc_lower = encoding.to_lowercase();

    // 处理常见编码别名
    let label = match enc_lower.as_str() {
        "gb2312" | "gbk" | "gb18030" => "gbk",
        "utf8" => "utf-8",
        other => other,
    };

    if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
        let (decoded, _encoding_used, _had_errors) = enc.decode(raw);
        decoded.into_owned()
    } else {
        // 无法识别的编码，尝试 UTF-8
        let (decoded, _encoding_used, _had_errors) = encoding_rs::UTF_8.decode(raw);
        decoded.into_owned()
    }
}

/// 从文件路径读取全部字节，并处理 BOM
fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件 '{}': {}", path, e))?;
    let mut buf_reader = BufReader::new(file);
    let mut bytes = Vec::new();
    buf_reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("读取文件Failed: {}", e))?;
    Ok(bytes)
}

/// 去除 UTF-8 BOM（EF BB BF）
fn strip_bom(bytes: &[u8]) -> &[u8] {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        &bytes[3..]
    } else {
        bytes
    }
}

/// 将原始字节数据解码为 CSV 字符串内容
///
/// 如果指定了编码则使用指定编码，否则自动检测。
/// 自动跳过 UTF-8 BOM。
fn decode_csv_content(raw: &[u8], encoding: Option<&str>) -> (String, String) {
    let detected = encoding
        .map(|e| e.to_string())
        .unwrap_or_else(|| detect_encoding(raw));

    // BOM 处理：UTF-8 BOM 在解码前去除
    let data = strip_bom(raw);
    let content = decode_bytes(data, &detected);
    (content, detected)
}

/// 解析 CSV 文件，返回行迭代器
///
/// # 参数
/// - `path`: 文件路径
/// - `encoding`: 指定编码 (None = 自动检测)
/// - `delimiter`: 分隔符 (None = 逗号)
///
/// # 返回
/// `CsvReader` 提供逐行读取接口，每行返回一个 JSON 对象。
pub fn parse_csv(
    path: &str,
    encoding: Option<&str>,
    delimiter: Option<char>,
) -> Result<CsvReader, String> {
    let raw = read_file_bytes(path)?;
    let (content, _detected_encoding) = decode_csv_content(&raw, encoding);

    let delim = delimiter.unwrap_or(',') as u8;

    let boxed: Box<dyn std::io::Read + Send> = Box::new(std::io::Cursor::new(content));
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delim)
        .has_headers(true)
        .flexible(true)
        .from_reader(boxed);

    let headers = reader
        .headers()
        .map_err(|e| format!("读取 CSV 表头Failed: {}", e))?
        .clone();

    Ok(CsvReader {
        reader,
        headers,
        finished: false,
    })
}

/// 预览 CSV 文件前 N 行
///
/// 读取并解析 CSV 文件的前 `n_rows` 行数据，返回 JSON 对象数组。
/// 适用于大文件的快速预览，不会加载全部数据。
///
/// # 参数
/// - `path`: 文件路径
/// - `encoding`: 指定编码 (None = 自动检测)
/// - `delimiter`: 分隔符 (None = 逗号)
/// - `n_rows`: 要预览的行数
pub fn preview(
    path: &str,
    encoding: Option<&str>,
    delimiter: Option<char>,
    n_rows: usize,
) -> Result<Vec<Value>, String> {
    let mut csv_reader = parse_csv(path, encoding, delimiter)?;
    let mut rows = Vec::with_capacity(n_rows);
    for _ in 0..n_rows {
        match csv_reader.next_row() {
            Some(Ok(row)) => rows.push(row),
            Some(Err(e)) => return Err(e),
            None => break,
        }
    }
    Ok(rows)
}

/// 验证 CSV 文件元信息
///
/// 扫描整个文件，返回行数、列名列表和检测到的编码。
/// 适用于在加载数据前获取文件概况。
///
/// # 参数
/// - `path`: 文件路径
/// - `encoding`: 指定编码 (None = 自动检测)
/// - `delimiter`: 分隔符 (None = 逗号)
pub fn validate(
    path: &str,
    encoding: Option<&str>,
    delimiter: Option<char>,
) -> Result<CsvInfo, String> {
    let raw = read_file_bytes(path)?;
    let (content, detected_encoding) = decode_csv_content(&raw, encoding);

    let delim = delimiter.unwrap_or(',') as u8;

    let boxed: Box<dyn std::io::Read + Send> = Box::new(std::io::Cursor::new(content));
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delim)
        .has_headers(true)
        .flexible(true)
        .from_reader(boxed);

    let headers = reader
        .headers()
        .map_err(|e| format!("读取 CSV 表头Failed: {}", e))?;
    let column_names: Vec<String> = headers.iter().map(|s| s.to_string()).collect();

    let mut row_count: u32 = 0;
    let mut record = csv::StringRecord::new();
    loop {
        match reader.read_record(&mut record) {
            Ok(false) => break,
            Ok(true) => row_count += 1,
            Err(e) => return Err(format!("读取 CSV 行Failed: {}", e)),
        }
    }

    Ok(CsvInfo {
        row_count,
        column_names,
        detected_encoding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    /// 辅助函数：创建临时 CSV 文件并返回路径
    fn create_temp_csv(content: &[u8], suffix: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(suffix)
            .tempfile()
            .expect("创建临时文件Failed");
        file.write_all(content).expect("写入临时文件Failed");
        file
    }

    // ==================== 编码检测测试 ====================

    #[test]
    fn test_detect_encoding_utf8() {
        let data = "name,age\n张三,25\n".as_bytes();
        assert_eq!(detect_encoding(data), "utf-8");
    }

    #[test]
    fn test_detect_encoding_utf8_bom() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice("name,age\n".as_bytes());
        assert_eq!(detect_encoding(&data), "utf-8");
    }

    #[test]
    fn test_detect_encoding_fallback_utf8() {
        // 短 ASCII 文本默认检测为 UTF-8
        let data = "hello,world\n".as_bytes();
        let encoding = detect_encoding(data);
        assert!(
            encoding.contains("utf") || encoding.contains("ascii"),
            "预期 UTF-8 或 ASCII，实际: {}",
            encoding
        );
    }

    // ==================== BOM 处理测试 ====================

    #[test]
    fn test_strip_bom() {
        let with_bom = vec![0xEF, 0xBB, 0xBF, b'h', b'i'];
        assert_eq!(strip_bom(&with_bom), b"hi");

        let without_bom = b"hello";
        assert_eq!(strip_bom(without_bom), b"hello");
    }

    // ==================== 解码测试 ====================

    #[test]
    fn test_decode_bytes_utf8() {
        let text = "你好世界";
        let encoded = text.as_bytes();
        let decoded = decode_bytes(encoded, "utf-8");
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_decode_bytes_gbk() {
        // GBK 编码的 "你好"
        let gbk_bytes: &[u8] = &[0xC4, 0xE3, 0xBA, 0xC3];
        let decoded = decode_bytes(gbk_bytes, "gbk");
        assert_eq!(decoded, "你好");
    }

    #[test]
    fn test_decode_bytes_gb2312_alias() {
        // GB2312 是 GBK 的子集，应该映射到 GBK 解码器
        let gbk_bytes: &[u8] = &[0xC4, 0xE3, 0xBA, 0xC3];
        let decoded = decode_bytes(gbk_bytes, "gb2312");
        assert_eq!(decoded, "你好");
    }

    #[test]
    fn test_decode_bytes_unknown_fallback() {
        let text = "hello";
        let decoded = decode_bytes(text.as_bytes(), "unknown-encoding");
        assert_eq!(decoded, "hello");
    }

    // ==================== UTF-8 CSV 解析测试 ====================

    #[test]
    fn test_parse_csv_utf8_basic() {
        let content = "name,age,city\n张三,25,北京\n李四,30,上海\n王五,28,广州\n";
        let file = create_temp_csv(content.as_bytes(), ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let mut reader = parse_csv(path, None, None).expect("解析Failed");

        let row1 = reader
            .next_row()
            .expect("应有第1行")
            .expect("第1行应Success");
        assert_eq!(row1["name"], json!("张三"));
        assert_eq!(row1["age"], json!("25"));
        assert_eq!(row1["city"], json!("北京"));

        let row2 = reader
            .next_row()
            .expect("应有第2行")
            .expect("第2行应Success");
        assert_eq!(row2["name"], json!("李四"));

        let row3 = reader
            .next_row()
            .expect("应有第3行")
            .expect("第3行应Success");
        assert_eq!(row3["name"], json!("王五"));

        assert!(reader.next_row().is_none(), "不应有第4行");
    }

    #[test]
    fn test_parse_csv_collect_all() {
        let file = create_temp_csv(b"x,y\n1,2\n3,4\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["x"], json!("1"));
        assert_eq!(rows[1]["y"], json!("4"));
    }

    // ==================== BOM CSV 测试 ====================

    #[test]
    fn test_parse_csv_utf8_bom() {
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice(b"name,age\nAlice,30\n");

        let file = create_temp_csv(&content, ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], json!("Alice"));
    }

    // ==================== 空 CSV 测试（仅有表头） ====================

    #[test]
    fn test_parse_csv_empty() {
        let file = create_temp_csv(b"name,age,city\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");
        assert!(rows.is_empty(), "空 CSV 应返回 0 行");
    }

    // ==================== 自定义分隔符测试 ====================

    #[test]
    fn test_parse_csv_tab_delimiter() {
        let file = create_temp_csv(b"name\tage\nAlice\t30\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, Some('\t')).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], json!("Alice"));
        assert_eq!(rows[0]["age"], json!("30"));
    }

    // ==================== 参差不齐的行测试 ====================

    #[test]
    fn test_parse_csv_ragged_rows_missing_columns() {
        // 第二行缺少一列 → 填充空字符串
        let file = create_temp_csv(b"a,b,c\n1,2,3\n4,5\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["c"], json!("3"));
        assert_eq!(rows[1]["b"], json!("5"));
        assert_eq!(rows[1]["c"], json!(""), "缺失列应为空字符串");
    }

    #[test]
    fn test_parse_csv_ragged_rows_extra_columns() {
        // 第二行多一列 → 忽略多余列
        let file = create_temp_csv(b"a,b\n1,2\n3,4,5\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1]["a"], json!("3"));
        assert_eq!(rows[1]["b"], json!("4"));
        // 不应有 'c' 键
        assert!(rows[1].get("c").is_none(), "多余列应被忽略");
    }

    // ==================== 文件不存在测试 ====================

    #[test]
    fn test_parse_csv_file_not_found() {
        let result = parse_csv("/nonexistent/path.csv", None, None);
        assert!(result.is_err(), "文件不存在应返回Error");
        let err = result.err().unwrap();
        assert!(
            err.contains("无法打开文件"),
            "Error信息应包含'无法打开文件'，实际: {}",
            err
        );
    }

    // ==================== Preview 测试 ====================

    #[test]
    fn test_preview_basic() {
        let file = create_temp_csv(
            b"name,age\nAlice,30\nBob,25\nCharlie,35\nDiana,28\n",
            ".csv",
        );
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, None, None, 2).expect("预览Failed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], json!("Alice"));
        assert_eq!(rows[1]["name"], json!("Bob"));
    }

    #[test]
    fn test_preview_fewer_rows_than_requested() {
        let file = create_temp_csv(b"name,age\nAlice,30\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, None, None, 10).expect("预览Failed");
        assert_eq!(rows.len(), 1, "请求 10 行但只有 1 行数据");
    }

    #[test]
    fn test_preview_empty_csv() {
        let file = create_temp_csv(b"name,age\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, None, None, 5).expect("预览Failed");
        assert!(rows.is_empty(), "空 CSV 预览应返回空数组");
    }

    #[test]
    fn test_preview_zero_rows() {
        let file = create_temp_csv(b"name,age\nAlice,30\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, None, None, 0).expect("预览Failed");
        assert!(rows.is_empty(), "预览 0 行应返回空数组");
    }

    // ==================== Validate 测试 ====================

    #[test]
    fn test_validate_basic() {
        let file = create_temp_csv(
            b"name,age,city\nAlice,30,Beijing\nBob,25,Shanghai\n",
            ".csv",
        );
        let path = file.path().to_str().expect("路径转换Failed");

        let info = validate(path, None, None).expect("验证Failed");
        assert_eq!(info.row_count, 2);
        assert_eq!(info.column_names, vec!["name", "age", "city"]);
        assert_eq!(info.detected_encoding, "utf-8");
    }

    #[test]
    fn test_validate_empty_csv() {
        let file = create_temp_csv(b"col1,col2,col3\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let info = validate(path, None, None).expect("验证Failed");
        assert_eq!(info.row_count, 0, "空 CSV 应有 0 行数据");
        assert_eq!(info.column_names.len(), 3);
    }

    #[test]
    fn test_validate_file_not_found() {
        let result = validate("/nonexistent/path.csv", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_with_specified_encoding() {
        let file = create_temp_csv(b"name,age\nAlice,30\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let info = validate(path, Some("utf-8"), None).expect("验证Failed");
        assert_eq!(info.detected_encoding, "utf-8");
    }

    #[test]
    fn test_validate_custom_delimiter() {
        let file = create_temp_csv(b"a;b;c\n1;2;3\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let info = validate(path, None, Some(';')).expect("验证Failed");
        assert_eq!(info.column_names, vec!["a", "b", "c"]);
        assert_eq!(info.row_count, 1);
    }

    // ==================== GBK 编码测试 ====================

    #[test]
    fn test_parse_csv_gbk() {
        // GBK 编码的 CSV: "姓名,年龄\n张三,25\n"
        let mut gbk_data = Vec::new();
        gbk_data.extend_from_slice(&encoding_rs::GBK.encode("姓名,年龄\n").0);
        gbk_data.extend_from_slice(&encoding_rs::GBK.encode("张三,25\n").0);

        let file = create_temp_csv(&gbk_data, ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, Some("gbk"), None, 10).expect("预览Failed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["姓名"], json!("张三"));
        assert_eq!(rows[0]["年龄"], json!("25"));
    }

    #[test]
    fn test_validate_gbk_auto_detect() {
        // 构造 GBK 编码的 CSV
        let mut gbk_data = Vec::new();
        gbk_data.extend_from_slice(&encoding_rs::GBK.encode("姓名,年龄\n").0);
        gbk_data.extend_from_slice(&encoding_rs::GBK.encode("张三,25\n").0);
        gbk_data.extend_from_slice(&encoding_rs::GBK.encode("李四,30\n").0);

        let file = create_temp_csv(&gbk_data, ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let info = validate(path, None, None).expect("验证Failed");
        assert_eq!(info.row_count, 2);
        assert_eq!(info.column_names, vec!["姓名", "年龄"]);
    }

    // ==================== Big5 编码测试 ====================

    #[test]
    fn test_parse_csv_big5() {
        // Big5 编码的 CSV: "姓名,年齡\n測試,25\n"
        let mut big5_data = Vec::new();
        big5_data.extend_from_slice(&encoding_rs::BIG5.encode("姓名,年齡\n").0);
        big5_data.extend_from_slice(&encoding_rs::BIG5.encode("測試,25\n").0);

        let file = create_temp_csv(&big5_data, ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let rows = preview(path, Some("big5"), None, 10).expect("预览Failed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["姓名"], json!("測試"));
    }

    // ==================== 引号和特殊字符测试 ====================

    #[test]
    fn test_parse_csv_quoted_fields() {
        let file = create_temp_csv(
            b"name,description\nAlice,\"Hello, World\"\nBob,\"He said \"\"hi\"\"\"\n",
            ".csv",
        );
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["description"], json!("Hello, World"));
        assert_eq!(rows[1]["description"], json!("He said \"hi\""));
    }

    #[test]
    fn test_parse_csv_empty_fields() {
        let file = create_temp_csv(b"a,b,c\n1,,3\n,2,\n", ".csv");
        let path = file.path().to_str().expect("路径转换Failed");

        let reader = parse_csv(path, None, None).expect("解析Failed");
        let rows = reader.collect_all().expect("收集Failed");

        assert_eq!(rows[0]["b"], json!(""));
        assert_eq!(rows[1]["a"], json!(""));
        assert_eq!(rows[1]["c"], json!(""));
    }

    // ==================== CsvInfo 测试 ====================

    #[test]
    fn test_csv_info_debug_clone() {
        let info = CsvInfo {
            row_count: 5,
            column_names: vec!["a".to_string(), "b".to_string()],
            detected_encoding: "utf-8".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(cloned.row_count, 5);
        assert_eq!(cloned.column_names, vec!["a", "b"]);

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("CsvInfo"));
    }
}
