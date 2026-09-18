#!/usr/bin/env node

/**
 * scripts/prepare-changelog.js
 *
 * 检查并准备 CHANGELOG.md 中的版本更新说明：
 * 1. 若 CHANGELOG.md 中已存在目标版本（如 ## [0.4.0-beta]），直接提取该小节说明；
 * 2. 若目标版本不存在，但 ## [Unreleased] 中有内容，则自动将 [Unreleased] 提升为
 *    ## [<version>] - YYYY-MM-DD，并在顶部重新保留空的 ## [Unreleased] 小节写回文件；
 * 3. 若目标版本不存在且 [Unreleased] 为空，则输出错误并以退出码 1 拒绝发布。
 *
 * 标准输出 (stdout): 提取到的 Markdown 格式更新说明
 * 标准错误 (stderr): 提示信息与错误日志
 *
 * 用法:
 *   node scripts/prepare-changelog.js [version] [--check-only] [--quiet]
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const projectRoot = path.resolve(__dirname, '..');
const changelogPath = path.resolve(projectRoot, 'CHANGELOG.md');
const packageJsonPath = path.resolve(projectRoot, 'package.json');

const args = process.argv.slice(2);
const checkOnly = args.includes('--check-only');
const quiet = args.includes('--quiet');
const versionArg = args.find((a) => !a.startsWith('--'));

function log(msg) {
  if (!quiet) {
    console.error(msg);
  }
}

function error(msg) {
  console.error(msg);
}

// 1. 获取目标版本号
let version = versionArg;
if (!version && fs.existsSync(packageJsonPath)) {
  try {
    const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    version = pkg.version;
  } catch (e) {
    error(`❌ 读取 package.json 失败: ${e.message}`);
    process.exit(1);
  }
}

if (!version) {
  error('❌ 未指定版本号且无法从 package.json 读取');
  process.exit(1);
}

const cleanVersion = version.replace(/^v/, '');
const cleanVerEscaped = cleanVersion.replace(/\./g, '\\.');

// 2. 读取 CHANGELOG.md
if (!fs.existsSync(changelogPath)) {
  error(`❌ 未找到 CHANGELOG.md: ${changelogPath}`);
  process.exit(1);
}

const md = fs.readFileSync(changelogPath, 'utf8');
const eol = md.includes('\r\n') ? '\r\n' : '\n';

// 3. 检查是否已有该版本的条目
const existingRegex = new RegExp(
  '## \\[(?:v)?' + cleanVerEscaped + '\\][^\r\n]*(?:\r?\n)([\\s\\S]*?)(?=(?:\r?\n)## |$)'
);
const existingMatch = md.match(existingRegex);

if (existingMatch && existingMatch[1] && existingMatch[1].trim()) {
  const notes = existingMatch[1].trim();
  log(`✓ CHANGELOG.md 已存在版本 [${cleanVersion}] 的更新说明`);
  process.stdout.write(notes);
  process.exit(0);
}

// 4. 检查 [Unreleased] 中是否有实质内容
const unreleasedRegex = /## \[Unreleased\][^\r\n]*(?:\r?\n)([\s\S]*?)(?=(?:\r?\n)## |$)/;
const unreleasedMatch = md.match(unreleasedRegex);
const unreleasedContent = unreleasedMatch && unreleasedMatch[1] ? unreleasedMatch[1].trim() : '';

if (unreleasedContent) {
  if (checkOnly) {
    log(`ℹ️ CHANGELOG.md 中的 [Unreleased] 包含可用说明，可在发版时自动转换为 [${cleanVersion}]`);
    process.stdout.write(unreleasedContent);
    process.exit(0);
  }

  const today = new Date().toISOString().slice(0, 10);
  const newSection = `## [Unreleased]${eol}${eol}## [${cleanVersion}] - ${today}${eol}${eol}${unreleasedContent}${eol}`;
  const newMd = md.replace(unreleasedRegex, newSection);

  fs.writeFileSync(changelogPath, newMd, 'utf8');
  log(`✓ 已将 CHANGELOG.md 中的 [Unreleased] 自动转换为 [${cleanVersion}] - ${today}`);
  process.stdout.write(unreleasedContent);
  process.exit(0);
}

// 5. 既没有目标版本，[Unreleased] 也为空 -> 拒绝发布
error(`❌ CHANGELOG.md 中未找到版本 [${cleanVersion}] 的更新说明，且 [Unreleased] 为空，拒绝发布！`);
error(`   请先在 CHANGELOG.md 的 [Unreleased] 或 [${cleanVersion}] 小节下补充更新内容。`);
process.exit(1);
