/**
 * TLS 测试运行器 — 自动启动服务器、执行流程、报告结果
 *
 * 用法:
 *   node tests/run-tls-tests.mjs
 *
 * 先决条件:
 *   1. 证书已生成 (tests/certs/ 目录)
 *   2. mpe CLI 已编译 (cd src-tauri && cargo build --bin mpe)
 *   3. Node.js >= 18
 */

import { spawn, execSync } from 'child_process';
import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(__dirname, '..');
const CERT_DIR = resolve(PROJECT_ROOT, 'tests/certs');
const CLI_BIN = resolve(PROJECT_ROOT, 'src-tauri/target/debug/mpe.exe');
const FLOW_DIR = resolve(PROJECT_ROOT, 'tests/cli');
const SERVER_SCRIPT = resolve(PROJECT_ROOT, 'tests/https-test-server.mjs');
const PORT = 8443;

// =========================================================================
// 测试用例定义
// =========================================================================

const TESTS = [
  {
    id: 'tls_01_basic_ca_verify',
    name: '✅ 基础 CA 验证',
    desc: '配正确 CA 证书 → 预期 200 成功',
    flow: 'tls_01_basic_ca_verify.json',
    serverMode: 'standard',
    expectSuccess: true,
  },
  {
    id: 'tls_02_skip_verify',
    name: '✅ 跳过证书验证',
    desc: '不配 CA，开启跳过验证 → 预期 200 成功',
    flow: 'tls_02_skip_verify.json',
    serverMode: 'standard',
    expectSuccess: true,
  },
  {
    id: 'tls_03_mtls_auth',
    name: '✅ mTLS 双向认证',
    desc: '配 CA + 客户端证书+密钥 → 预期 200 成功',
    flow: 'tls_03_mtls_auth.json',
    serverMode: 'mtls',
    expectSuccess: true,
  },
  {
    id: 'tls_04_ca_missing',
    name: '❌ 无 CA 证书（预期失败）',
    desc: '不配任何 TLS 配置 → 预期握手失败',
    flow: 'tls_04_ca_missing.json',
    serverMode: 'standard',
    expectSuccess: false,
  },
  {
    id: 'tls_05_wrong_ca',
    name: '❌ 错误 CA 证书（预期失败）',
    desc: '配一个错误的 CA → 预期握手失败',
    flow: 'tls_05_wrong_ca.json',
    serverMode: 'standard',
    expectSuccess: false,
  },
];

// =========================================================================
// 辅助函数
// =========================================================================

/** 强制杀掉占用指定端口的所有进程 */
function killPort(port) {
  try {
    const cmd = `netstat -ano | Select-String ":${port}" | Select-String "LISTENING"`;
    const output = execSync(`powershell -Command "${cmd}"`, { encoding: 'utf-8' }).trim();
    if (!output) return;
    
    const pids = [...new Set(
      output.split('\n')
        .map(line => line.trim().split(/\s+/).pop())
        .filter(pid => /^\d+$/.test(pid))
    )];
    
    for (const pid of pids) {
      try {
        execSync(`taskkill /PID ${pid} /F /T`, { encoding: 'utf-8' });
        console.log(`    [cleanup] 已杀掉进程 PID=${pid} (端口 ${port})`);
      } catch { /* 忽略已结束的进程 */ }
    }
    // 等待端口释放
    return new Promise(r => setTimeout(r, 500));
  } catch { /* 无进程占用 */ }
}

/** 启动 HTTPS 测试服务器，返回进程引用 */
function startServer(mode) {
  return new Promise(async (resolvePromise) => {
    // 先清理端口
    await killPort(PORT);
    
    const args = [SERVER_SCRIPT];
    if (mode === 'mtls') args.push('--mtls');

    const proc = spawn('node', args, {
      cwd: PROJECT_ROOT,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: true,
    });

    proc._serverMode = mode;
    proc._serverPort = PORT;

    const timeout = setTimeout(() => {
      console.log(`    [server] 超时，继续...`);
      resolvePromise(proc);
    }, 5000);

    proc.stdout.on('data', (data) => {
      const text = data.toString();
      if (text.includes('启动成功')) {
        clearTimeout(timeout);
        resolvePromise(proc);
      }
    });
    proc.stderr.on('data', (data) => {
      console.error(`    [server-err] ${data.toString().trim()}`);
    });
  });
}

/** 运行单个流测试 */
async function runFlow(flowPath) {
  return new Promise((resolvePromise) => {
    const proc = spawn(CLI_BIN, ['run', '-f', flowPath], {
      cwd: PROJECT_ROOT,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: true,
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (d) => { stdout += d.toString(); });
    proc.stderr.on('data', (d) => { stderr += d.toString(); });

    proc.on('close', (code) => {
      try {
        // 找到完整的 JSON 对象（pretty-printed JSON 有多行）
        const fullOutput = stdout.trim();
        const startIdx = fullOutput.indexOf('{');
        if (startIdx < 0) {
          resolvePromise({ code, result: null, stderr: stderr.trim(), raw: fullOutput });
          return;
        }
        // 找到匹配的 } 来提取完整 JSON
        let depth = 0;
        let endIdx = startIdx;
        for (let i = startIdx; i < fullOutput.length; i++) {
          if (fullOutput[i] === '{') depth++;
          else if (fullOutput[i] === '}') depth--;
          if (depth === 0) { endIdx = i; break; }
        }
        const jsonStr = fullOutput.substring(startIdx, endIdx + 1);
        const result = JSON.parse(jsonStr);
        resolvePromise({ code, result, stderr: stderr.trim() });
      } catch (e) {
        resolvePromise({ code, result: null, stderr: stderr.trim(), raw: stdout.trim() });
      }
    });
  });
}

// =========================================================================
// 主流程
// =========================================================================

async function main() {
  console.log('='.repeat(60));
  console.log('  TLS 功能验证测试套件');
  console.log('='.repeat(60));
  console.log();

  // 检查先决条件
  if (!existsSync(CLI_BIN)) {
    console.error(`[错误] mpe CLI 未编译: ${CLI_BIN}`);
    console.error('请先运行: cd src-tauri && cargo build --bin mpe');
    process.exit(1);
  }
  if (!existsSync(resolve(CERT_DIR, 'ca.pem'))) {
    console.error(`[错误] 证书未生成: ${CERT_DIR}`);
    console.error('请先运行 certs 生成脚本');
    process.exit(1);
  }

  // 预清理
  await killPort(PORT);

  const results = { pass: 0, fail: 0 };
  let serverProc = null;
  let currentMode = null;

  try {
    for (const test of TESTS) {
      console.log(`──────────────────────────────────────────────`);
      console.log(`  测试: ${test.name}`);
      console.log(`  说明: ${test.desc}`);

      // 按需切换服务器模式
      if (test.serverMode !== currentMode) {
        if (serverProc) {
          console.log(`    停止旧服务器 (PID=${serverProc.pid})...`);
          serverProc.kill('SIGTERM');
          await killPort(PORT);
          serverProc = null;
        }
        console.log(`    启动服务器 (${test.serverMode} 模式)...`);
        serverProc = await startServer(test.serverMode);
        currentMode = test.serverMode;
        // 等服务器稳定
        await new Promise(r => setTimeout(r, 1500));
      }

      // 使用 flow 文件原始路径（证书路径是相对于项目根目录的）
      const flowPath = resolve(FLOW_DIR, test.flow);
      console.log(`    执行流程...`);

      const start = Date.now();
      const { code, result, stderr } = await runFlow(flowPath);
      const elapsed = Date.now() - start;

      // 从 node_results 中找到 HTTP 节点的实际输出（在 data 层内）
      const httpNodeResult = result?.data?.node_results?.find(nr => nr.node_type === 'http');
      const httpOutput = httpNodeResult?.output;
      // 检查 HTTP 节点的 success（而非 CLI 层面的 success）
      const gotSuccess = httpOutput?.success === true;
      const gotError = httpOutput?.error || httpNodeResult?.error?.message || '';
      const passed = gotSuccess === test.expectSuccess;

      if (passed) {
        results.pass++;
        const icon = test.expectSuccess ? '✅' : '❌';
        process.stdout.write(`  ${icon} 通过 (${elapsed}ms)`);
        if (!test.expectSuccess && gotError) {
          process.stdout.write(` — 错误: ${gotError.slice(0, 80)}`);
        }
        process.stdout.write('\n');
      } else {
        results.fail++;
        process.stdout.write(`  ❌ 失败! (${elapsed}ms)\n`);
        process.stdout.write(`     期望 success=${test.expectSuccess}, 实际 HTTP success=${gotSuccess}\n`);
        if (gotError) {
          process.stdout.write(`     错误: ${gotError.slice(0, 200)}\n`);
        }
      }
      console.log();
    }

  } finally {
    // 清理
    if (serverProc) {
      serverProc.kill('SIGTERM');
    }
    await killPort(PORT);
  }

  // 总结
  console.log('='.repeat(60));
  console.log(`  测试完成`);
  console.log(`  ✅ 通过: ${results.pass}`);
  console.log(`  ❌ 失败: ${results.fail}`);
  console.log(`  📊 总计: ${results.pass + results.fail}`);
  console.log('='.repeat(60));

  process.exit(results.fail > 0 ? 1 : 0);
}

main();
