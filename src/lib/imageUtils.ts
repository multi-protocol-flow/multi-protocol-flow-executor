/**
 * 图片处理与多模态预压缩工具
 */

/**
 * 将图片文件压缩并转换为 Base64 Data URL
 *
 * @param file 图片文件或 Blob 对象
 * @param maxDimension 最大长宽尺寸 (默认 2048px，视觉模型推荐的最佳识别尺寸)
 * @param quality JPEG 压缩质量 (0.0 ~ 1.0，默认 0.85)
 * @returns Base64 Data URL 字符串
 */
export async function compressAndConvertToBase64(
  file: File | Blob,
  maxDimension = 2048,
  quality = 0.85,
): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = (e) => {
      const img = new Image();

      img.onload = () => {
        let width = img.width;
        let height = img.height;

        // 如果超出最大尺寸限制，等比例缩小
        if (width > maxDimension || height > maxDimension) {
          if (width > height) {
            height = Math.round((height * maxDimension) / width);
            width = maxDimension;
          } else {
            width = Math.round((width * maxDimension) / height);
            height = maxDimension;
          }
        }

        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;

        const ctx = canvas.getContext('2d');
        if (!ctx) {
          // 如果 canvas 无法获取 context，回退直接返回原始 base64
          resolve(e.target?.result as string);
          return;
        }

        ctx.drawImage(img, 0, 0, width, height);

        // 如果是 PNG 或带透明通道的图片，优先保持 PNG，否则用 JPEG 压缩
        const isPng = file.type === 'image/png';
        const mimeType = isPng ? 'image/png' : 'image/jpeg';
        const dataUrl = canvas.toDataURL(mimeType, quality);

        resolve(dataUrl);
      };

      img.onerror = () => {
        reject(new Error('Failed to load image for compression'));
      };

      img.src = e.target?.result as string;
    };

    reader.onerror = () => {
      reject(reader.error || new Error('Failed to read file'));
    };

    reader.readAsDataURL(file);
  });
}

/**
 * 从剪贴板事件中提取图片并转换为 Base64 列表
 */
export async function extractImagesFromClipboard(
  clipboardData: DataTransfer | null,
): Promise<string[]> {
  if (!clipboardData) return [];

  const imageFiles: File[] = [];
  const items = Array.from(clipboardData.items || []);

  for (const item of items) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile();
      if (file) {
        imageFiles.push(file);
      }
    }
  }

  const results: string[] = [];
  for (const file of imageFiles) {
    try {
      const base64 = await compressAndConvertToBase64(file);
      results.push(base64);
    } catch {
      // 忽略单张图片处理异常
    }
  }

  return results;
}

/**
 * 从文件拖拽事件中提取图片并转换为 Base64 列表
 */
export async function extractImagesFromDrop(
  dataTransfer: DataTransfer | null,
): Promise<string[]> {
  if (!dataTransfer) return [];

  const imageFiles: File[] = [];
  const files = Array.from(dataTransfer.files || []);

  for (const file of files) {
    if (file.type.startsWith('image/')) {
      imageFiles.push(file);
    }
  }

  const results: string[] = [];
  for (const file of imageFiles) {
    try {
      const base64 = await compressAndConvertToBase64(file);
      results.push(base64);
    } catch {
      // 忽略异常
    }
  }

  return results;
}
