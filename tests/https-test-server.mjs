/**
 * TLS 测试 HTTPS 服务器
 *
 * 用法:
 *   node tests/https-test-server.mjs [--mtls]
 *
 * 普通模式（默认）: 只验证服务端证书，不要求客户端证书
 *   node tests/https-test-server.mjs
 *
 * mTLS 模式: 要求客户端提供由同一 CA 签发的证书
 *   node tests/https-test-server.mjs --mtls
 *
 * 支持端点:
 *   GET /api/echo       — 返回请求信息
 *   GET /api/status/503 — 返回指定状态码
 */

import https from 'https';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const certDir = path.join(__dirname, 'certs');
const port = 8443;
const isMTLS = process.argv.includes('--mtls');

// 加载证书
const options = {
  key: fs.readFileSync(path.join(certDir, 'server.key')),
  cert: fs.readFileSync(path.join(certDir, 'server.pem')),
};

if (isMTLS) {
  // mTLS 模式: 要求客户端证书，且由同一 CA 签发
  options.requestCert = true;
  options.rejectUnauthorized = true;
  options.ca = [fs.readFileSync(path.join(certDir, 'ca.pem'))];
  console.log('[TLS Test Server] mTLS 模式已启用 — 需要客户端证书');
} else {
  console.log('[TLS Test Server] 标准模式 — 仅服务端证书');
}

const server = https.createServer(options, (req, res) => {
  const clientCert = req.socket.getPeerCertificate();
  const certInfo = clientCert?.subject
    ? { CN: clientCert.subject.CN, issuer: clientCert.issuer?.CN }
    : null;

  // 解析 URL 路径
  const url = new URL(req.url, `https://localhost:${port}`);
  const statusMatch = url.pathname.match(/^\/api\/status\/(\d+)$/);
  const statusCode = statusMatch ? parseInt(statusMatch[1], 10) : 200;

  const body = JSON.stringify({
    success: statusCode >= 200 && statusCode < 300,
    message: `TLS 测试服务器响应 (${isMTLS ? 'mTLS' : '标准'} 模式)`,
    method: req.method,
    path: url.pathname,
    status: statusCode,
    client_cert: certInfo,
    headers: req.headers,
    tls_mode: isMTLS ? 'mtls' : 'standard',
    server_info: {
      protocol: req.socket.getProtocol?.() || 'unknown',
      authorized: req.socket.authorized ?? null,
    },
  });

  res.writeHead(statusCode, {
    'Content-Type': 'application/json',
    'X-TLS-Test': 'true',
    'X-TLS-Mode': isMTLS ? 'mtls' : 'standard',
  });
  res.end(body);
});

server.listen(port, () => {
  console.log(`[TLS Test Server] 启动成功: https://localhost:${port}`);
  console.log(`[TLS Test Server] 测试端点:`);
  console.log(`  GET https://localhost:${port}/api/echo`);
  console.log(`  GET https://localhost:${port}/api/status/503`);
  console.log(`[TLS Test Server] 按 Ctrl+C 停止`);
});
