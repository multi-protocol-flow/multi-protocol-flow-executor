#!/usr/bin/env node

/**
 * scripts/inject-version.js
 *
 * 同步将版本号注入到项目所有声明版本的文件中：
 * 1. package.json
 * 2. src-tauri/tauri.conf.json
 * 3. src-tauri/Cargo.toml (package.version)
 * 4. src-tauri/crates/mpe-cli/Cargo.toml (package.version)
 *
 * 用法:
 *   node scripts/inject-version.js <version>
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');

const versionArg = process.argv[2];
if (!versionArg) {
  console.error('❌ 用法: node scripts/inject-version.js <version>');
  process.exit(1);
}

const version = versionArg.replace(/^v/, '');

// 1. package.json
const pkgPath = path.join(projectRoot, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`  ✓ package.json: ${version}`);

// 2. src-tauri/tauri.conf.json
const tauriPath = path.join(projectRoot, 'src-tauri/tauri.conf.json');
const tauri = JSON.parse(fs.readFileSync(tauriPath, 'utf8'));
tauri.version = version;
fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + '\n');
console.log(`  ✓ src-tauri/tauri.conf.json: ${version}`);

// 3. src-tauri/Cargo.toml
const cargoPath = path.join(projectRoot, 'src-tauri/Cargo.toml');
let cargo = fs.readFileSync(cargoPath, 'utf8');
cargo = cargo.replace(/((\r?\n|^)\[package\][\s\S]*?version\s*=\s*)"[^"]*"/, `$1"${version}"`);
fs.writeFileSync(cargoPath, cargo);
console.log(`  ✓ src-tauri/Cargo.toml: ${version}`);

// 4. src-tauri/crates/mpe-cli/Cargo.toml
const cliCargoPath = path.join(projectRoot, 'src-tauri/crates/mpe-cli/Cargo.toml');
let cliCargo = fs.readFileSync(cliCargoPath, 'utf8');
cliCargo = cliCargo.replace(/((\r?\n|^)\[package\][\s\S]*?version\s*=\s*)"[^"]*"/, `$1"${version}"`);
fs.writeFileSync(cliCargoPath, cliCargo);
console.log(`  ✓ src-tauri/crates/mpe-cli/Cargo.toml: ${version}`);
