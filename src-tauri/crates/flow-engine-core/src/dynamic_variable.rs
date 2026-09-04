//! 动态变量生成器 (Dynamic Variables)
//!
//! 提供类似 Postman 的轻量级动态变量生成能力（如 `{{$guid}}`、`{{$timestamp}}`、`{{$randomInt}}` 等）。
//! 所有变量均在内存中根据内置词表及随机数生成器即时生成，无沉重三方依赖。

use chrono::{Duration, SecondsFormat, Utc};
use rand::seq::SliceRandom;
use rand::Rng;

/// 支持的所有动态变量名称列表
pub const DYNAMIC_VARIABLE_NAMES: &[&str] = &[
    // 通用标识
    "$guid",
    "$uuid",
    "$randomUUID",
    // 日期时间
    "$timestamp",
    "$timestampMs",
    "$isoTimestamp",
    "$randomDatePast",
    "$randomDateFuture",
    "$randomDateRecent",
    // 随机数值与格式
    "$randomInt",
    "$randomPrice",
    "$randomBoolean",
    "$randomAlphaNumeric",
    "$randomString",
    "$randomHexColor",
    "$randomColor",
    "$randomSemver",
    // 网络通信
    "$randomIP",
    "$randomIPV6",
    "$randomMACAddress",
    "$randomPort",
    "$randomUrl",
    "$randomDomainName",
    // 人员身份
    "$randomFirstName",
    "$randomLastName",
    "$randomFullName",
    "$randomUserName",
    "$randomJobTitle",
    // 联系方式
    "$randomEmail",
    "$randomExampleEmail",
    "$randomPhoneNumber",
    "$randomPhone",
    // 文本数据
    "$randomWord",
    "$randomWords",
    // 地理商业
    "$randomCity",
    "$randomCountry",
    "$randomCountryCode",
    "$randomLatitude",
    "$randomLongitude",
    "$randomCurrencyCode",
    "$randomCompanyName",
    "$randomStreetAddress",
    "$randomPostalCode",
];

const FIRST_NAMES: &[&str] = &[
    "James", "Mary", "John", "Patricia", "Robert", "Jennifer", "Michael", "Linda",
    "William", "Elizabeth", "David", "Barbara", "Richard", "Susan", "Joseph", "Jessica",
    "Thomas", "Sarah", "Charles", "Karen", "Christopher", "Nancy", "Daniel", "Lisa",
    "Matthew", "Betty", "Anthony", "Margaret", "Mark", "Sandra", "Alex", "Sam",
    "Emma", "Liam", "Olivia", "Noah", "Sophia", "Lucas", "Mia", "Ethan",
];

const LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
    "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
    "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez",
    "Thompson", "White", "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson",
];

const DOMAINS: &[&str] = &[
    "gmail.com", "yahoo.com", "hotmail.com", "outlook.com", "icloud.com",
    "proton.me", "mail.com", "fastmail.com",
];

const EXAMPLE_DOMAINS: &[&str] = &["example.com", "example.org", "example.net"];

const WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
    "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et",
    "dolore", "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis",
    "nostrud", "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex",
    "ea", "commodo", "consequat", "duis", "aute", "irure", "in", "reprehenderit",
    "voluptate", "velit", "esse", "cillum", "fugiat", "nulla", "pariatur",
];

const CITIES: &[&str] = &[
    "New York", "London", "Tokyo", "Paris", "Berlin", "Sydney", "Beijing",
    "Singapore", "Toronto", "San Francisco", "Seattle", "Amsterdam", "Seoul",
    "Dubai", "Rome", "Madrid", "Chicago", "Boston", "Hong Kong", "Zurich",
];

const COUNTRIES: &[(&str, &str)] = &[
    ("United States", "US"),
    ("China", "CN"),
    ("United Kingdom", "GB"),
    ("Germany", "DE"),
    ("Japan", "JP"),
    ("France", "FR"),
    ("Australia", "AU"),
    ("Canada", "CA"),
    ("Singapore", "SG"),
    ("Netherlands", "NL"),
    ("South Korea", "KR"),
    ("Switzerland", "CH"),
    ("Spain", "ES"),
    ("Italy", "IT"),
    ("Brazil", "BR"),
];

const CURRENCIES: &[&str] = &["USD", "EUR", "CNY", "GBP", "JPY", "CAD", "AUD", "CHF", "HKD", "SGD"];

const COLORS: &[&str] = &[
    "red", "blue", "green", "yellow", "purple", "orange", "pink", "teal", "cyan", "indigo",
];

const COMPANIES: &[&str] = &[
    "Acme Corp", "Globex", "Initech", "Umbrella Corp", "Soylent Corp",
    "Hooli", "Pied Piper", "Stark Industries", "Wayne Enterprises", "Cyberdyne Systems",
];

const JOB_TITLES: &[&str] = &[
    "Software Engineer", "Product Manager", "DevOps Engineer", "Data Scientist",
    "UI/UX Designer", "QA Engineer", "Engineering Manager", "Technical Lead",
    "Security Analyst", "System Architect",
];

/// 检查给定变量名是否属于受支持的动态变量
pub fn is_dynamic_variable(name: &str) -> bool {
    DYNAMIC_VARIABLE_NAMES.contains(&name)
}

/// 解析动态变量并返回其生成后的字符串值
///
/// 若变量名不在受支持列表中，返回 `None`。
pub fn resolve_dynamic_variable(name: &str) -> Option<String> {
    let mut rng = rand::thread_rng();

    match name {
        // 通用标识
        "$guid" | "$uuid" | "$randomUUID" => Some(uuid::Uuid::new_v4().to_string()),

        // 日期时间
        "$timestamp" => Some(Utc::now().timestamp().to_string()),
        "$timestampMs" => Some(Utc::now().timestamp_millis().to_string()),
        "$isoTimestamp" => Some(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
        "$randomDatePast" => {
            let days = rng.gen_range(1..=365);
            let dt = Utc::now() - Duration::days(days);
            Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        }
        "$randomDateFuture" => {
            let days = rng.gen_range(1..=365);
            let dt = Utc::now() + Duration::days(days);
            Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        }
        "$randomDateRecent" => {
            let days = rng.gen_range(1..=30);
            let dt = Utc::now() - Duration::days(days);
            Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        }

        // 随机数值与格式
        "$randomInt" => Some(rng.gen_range(0..=1000).to_string()),
        "$randomPrice" => {
            let price: f64 = rng.gen_range(1.0..=999.99);
            Some(format!("{:.2}", price))
        }
        "$randomBoolean" => Some(if rng.gen_bool(0.5) { "true" } else { "false" }.to_string()),
        "$randomAlphaNumeric" => {
            let s: String = (0..8)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect();
            Some(s)
        }
        "$randomString" => {
            let s: String = (0..8)
                .map(|_| (b'a' + rng.gen_range(0..26)) as char)
                .collect();
            Some(s)
        }
        "$randomHexColor" => {
            let hex_val: u32 = rng.gen_range(0..=0xffffff);
            Some(format!("#{:06x}", hex_val))
        }
        "$randomColor" => Some(
            COLORS
                .choose(&mut rng)
                .copied()
                .unwrap_or("blue")
                .to_string(),
        ),
        "$randomSemver" => {
            let major = rng.gen_range(0..=5);
            let minor = rng.gen_range(0..=20);
            let patch = rng.gen_range(0..=99);
            Some(format!("{}.{}.{}", major, minor, patch))
        }

        // 网络通信
        "$randomIP" => {
            let o1 = rng.gen_range(1..=254);
            let o2 = rng.gen_range(0..=255);
            let o3 = rng.gen_range(0..=255);
            let o4 = rng.gen_range(1..=254);
            Some(format!("{}.{}.{}.{}", o1, o2, o3, o4))
        }
        "$randomIPV6" => Some(format!(
            "{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}",
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>()
        )),
        "$randomMACAddress" => Some(format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>(),
            rng.gen::<u8>()
        )),
        "$randomPort" => Some(rng.gen_range(1024..=65535).to_string()),
        "$randomUrl" => {
            let domain = DOMAINS.choose(&mut rng).copied().unwrap_or("example.com");
            let word = WORDS.choose(&mut rng).copied().unwrap_or("api");
            Some(format!("https://{}/{}", domain, word))
        }
        "$randomDomainName" => Some(
            DOMAINS
                .choose(&mut rng)
                .copied()
                .unwrap_or("example.com")
                .to_string(),
        ),

        // 人员身份
        "$randomFirstName" => Some(
            FIRST_NAMES
                .choose(&mut rng)
                .copied()
                .unwrap_or("Alex")
                .to_string(),
        ),
        "$randomLastName" => Some(
            LAST_NAMES
                .choose(&mut rng)
                .copied()
                .unwrap_or("Smith")
                .to_string(),
        ),
        "$randomFullName" => {
            let first = FIRST_NAMES.choose(&mut rng).copied().unwrap_or("Alex");
            let last = LAST_NAMES.choose(&mut rng).copied().unwrap_or("Smith");
            Some(format!("{} {}", first, last))
        }
        "$randomUserName" => {
            let first = FIRST_NAMES.choose(&mut rng).copied().unwrap_or("alex");
            let num = rng.gen_range(10..=9999);
            Some(format!("{}{}", first.to_lowercase(), num))
        }
        "$randomJobTitle" => Some(
            JOB_TITLES
                .choose(&mut rng)
                .copied()
                .unwrap_or("Developer")
                .to_string(),
        ),

        // 联系方式
        "$randomEmail" => {
            let first = FIRST_NAMES.choose(&mut rng).copied().unwrap_or("alex");
            let last = LAST_NAMES.choose(&mut rng).copied().unwrap_or("smith");
            let domain = DOMAINS.choose(&mut rng).copied().unwrap_or("example.com");
            let num = rng.gen_range(1..=99);
            Some(format!(
                "{}.{}{}@{}",
                first.to_lowercase(),
                last.to_lowercase(),
                num,
                domain
            ))
        }
        "$randomExampleEmail" => {
            let first = FIRST_NAMES.choose(&mut rng).copied().unwrap_or("alex");
            let last = LAST_NAMES.choose(&mut rng).copied().unwrap_or("smith");
            let domain = EXAMPLE_DOMAINS
                .choose(&mut rng)
                .copied()
                .unwrap_or("example.com");
            let num = rng.gen_range(1..=99);
            Some(format!(
                "{}.{}{}@{}",
                first.to_lowercase(),
                last.to_lowercase(),
                num,
                domain
            ))
        }
        "$randomPhoneNumber" | "$randomPhone" => {
            let area = rng.gen_range(200..=999);
            let prefix = rng.gen_range(200..=999);
            let line = rng.gen_range(1000..=9999);
            Some(format!("+1-{:03}-{:03}-{:04}", area, prefix, line))
        }

        // 文本数据
        "$randomWord" => Some(
            WORDS
                .choose(&mut rng)
                .copied()
                .unwrap_or("lorem")
                .to_string(),
        ),
        "$randomWords" => {
            let count = rng.gen_range(3..=5);
            let words: Vec<&str> = (0..count)
                .map(|_| WORDS.choose(&mut rng).copied().unwrap_or("word"))
                .collect();
            Some(words.join(" "))
        }

        // 地理商业
        "$randomCity" => Some(
            CITIES
                .choose(&mut rng)
                .copied()
                .unwrap_or("New York")
                .to_string(),
        ),
        "$randomCountry" => {
            let country = COUNTRIES
                .choose(&mut rng)
                .map(|(c, _)| *c)
                .unwrap_or("United States");
            Some(country.to_string())
        }
        "$randomCountryCode" => {
            let code = COUNTRIES
                .choose(&mut rng)
                .map(|(_, code)| *code)
                .unwrap_or("US");
            Some(code.to_string())
        }
        "$randomLatitude" => {
            let lat: f64 = rng.gen_range(-90.0..=90.0);
            Some(format!("{:.6}", lat))
        }
        "$randomLongitude" => {
            let lon: f64 = rng.gen_range(-180.0..=180.0);
            Some(format!("{:.6}", lon))
        }
        "$randomCurrencyCode" => Some(
            CURRENCIES
                .choose(&mut rng)
                .copied()
                .unwrap_or("USD")
                .to_string(),
        ),
        "$randomCompanyName" => Some(
            COMPANIES
                .choose(&mut rng)
                .copied()
                .unwrap_or("Acme Corp")
                .to_string(),
        ),
        "$randomStreetAddress" => {
            let num = rng.gen_range(100..=9999);
            let street = LAST_NAMES.choose(&mut rng).copied().unwrap_or("Main");
            Some(format!("{} {} St", num, street))
        }
        "$randomPostalCode" => {
            let code = rng.gen_range(10000..=99999);
            Some(format!("{:05}", code))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_dynamic_variables_resolve() {
        for &name in DYNAMIC_VARIABLE_NAMES {
            assert!(is_dynamic_variable(name), "Should recognize {}", name);
            let resolved = resolve_dynamic_variable(name);
            assert!(resolved.is_some(), "Should resolve {}", name);
            let val = match resolved {
                Some(v) => v,
                None => panic!("Failed to resolve {}", name),
            };
            assert!(!val.is_empty(), "Value for {} should not be empty", name);
        }
    }

    #[test]
    fn test_unknown_variable_returns_none() {
        assert!(!is_dynamic_variable("$notAValidVariable"));
        assert_eq!(resolve_dynamic_variable("$notAValidVariable"), None);
        assert!(!is_dynamic_variable("regularVariable"));
        assert_eq!(resolve_dynamic_variable("regularVariable"), None);
    }

    #[test]
    fn test_guid_format() {
        let val = resolve_dynamic_variable("$guid").expect("resolve $guid");
        assert_eq!(val.len(), 36);
        assert!(uuid::Uuid::parse_str(&val).is_ok());
    }

    #[test]
    fn test_timestamp_format() {
        let val = resolve_dynamic_variable("$timestamp").expect("resolve $timestamp");
        let ts: i64 = val.parse().expect("parse timestamp");
        assert!(ts > 1700000000);
    }

    #[test]
    fn test_timestamp_ms_format() {
        let val = resolve_dynamic_variable("$timestampMs").expect("resolve $timestampMs");
        let ts: i64 = val.parse().expect("parse timestampMs");
        assert!(ts > 1700000000000);
    }

    #[test]
    fn test_random_int_range() {
        let val = resolve_dynamic_variable("$randomInt").expect("resolve $randomInt");
        let num: i32 = val.parse().expect("parse int");
        assert!((0..=1000).contains(&num));
    }

    #[test]
    fn test_random_price_format() {
        let val = resolve_dynamic_variable("$randomPrice").expect("resolve $randomPrice");
        let price: f64 = val.parse().expect("parse price");
        assert!(price >= 1.0 && price <= 1000.0);
        assert!(val.contains('.'));
    }

    #[test]
    fn test_random_boolean() {
        let val = resolve_dynamic_variable("$randomBoolean").expect("resolve $randomBoolean");
        assert!(val == "true" || val == "false");
    }

    #[test]
    fn test_random_ip() {
        let val = resolve_dynamic_variable("$randomIP").expect("resolve $randomIP");
        let parts: Vec<&str> = val.split('.').collect();
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn test_random_email() {
        let val = resolve_dynamic_variable("$randomEmail").expect("resolve $randomEmail");
        assert!(val.contains('@'));
        assert!(val.contains('.'));
    }
}
