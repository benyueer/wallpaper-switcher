#![windows_subsystem = "windows"]

mod state;
mod scanner;
mod wallpaper;
mod config;

use state::AppState;
use scanner::load_images_from_dirs;
use wallpaper::apply_wallpaper;
use config::load_dirs;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process::Command;

use tray_item::{IconSource, TrayItem};
use std::path::PathBuf;


fn get_config_path() -> PathBuf {
    // 1️⃣ exe 同目录（生产环境）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let path = dir.join("config.txt");
            if path.exists() {
                return path;
            }
        }
    }

    // 2️⃣ fallback：工作目录（开发环境 cargo run）
    PathBuf::from("config.txt")
}

fn main() {
    let config_path = get_config_path();

    let dirs = load_dirs(config_path.to_str().unwrap());

    if dirs.is_empty() {
        eprintln!("No directories found in config.txt");
    }

    let images = load_images_from_dirs(&dirs);

  let state = Arc::new(Mutex::new(AppState::new(images)));

    // 自动循环线程
    {
        let state = state.clone();
        thread::spawn(move || {
            loop {
                {
                    let mut s = state.lock().unwrap();
                    if s.auto {
                        s.next();
                        apply_wallpaper(&s);
                    }
                }
                thread::sleep(Duration::from_secs(10));
            }
        });
    }

    let mut tray = TrayItem::new("Wallpaper", IconSource::Resource("IDI_ICON1")).unwrap();

    // 自动循环
    {
        let s = state.clone();
        tray.add_menu_item("自动循环", move || {
            let mut s = s.lock().unwrap();
            s.auto = true;
        }).unwrap();
    }

    // 停止循环
    {
        let s = state.clone();
        tray.add_menu_item("停止循环", move || {
            let mut s = s.lock().unwrap();
            s.auto = false;
        }).unwrap();
    }

    // 上一张
    {
        let s = state.clone();
        tray.add_menu_item("上一张", move || {
            let mut s = s.lock().unwrap();
            s.prev();
            apply_wallpaper(&s);
        }).unwrap();
    }

    // 下一张
    {
        let s = state.clone();
        tray.add_menu_item("下一张", move || {
            let mut s = s.lock().unwrap();
            s.next();
            apply_wallpaper(&s);
        }).unwrap();
    }

    // 打开第一个目录（默认）
    {
        let dir = dirs.get(0).cloned().unwrap_or_default();
        tray.add_menu_item("打开目录", move || {
            if !dir.is_empty() {
                let _ = Command::new("explorer").arg(&dir).spawn();
            }
        }).unwrap();
    }

    // 退出
    tray.add_menu_item("退出", || {
        std::process::exit(0);
    }).unwrap();

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}