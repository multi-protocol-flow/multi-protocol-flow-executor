//! Tauri 应用入口
//!
//! Windows 发布版本隐藏控制台窗口。


// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  app_lib::run();
}
