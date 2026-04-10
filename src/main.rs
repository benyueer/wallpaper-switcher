#![windows_subsystem = "windows"]

mod config;
mod scanner;
mod state;
mod wallpaper;

use config::load_dirs;
use scanner::load_images_from_dirs;
use state::{AppState, LoopTime, PlayOrder};
use wallpaper::apply_wallpaper;

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tray_icon::{
    TrayIconBuilder,
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

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

fn load_icon() -> tray_icon::Icon {
    let icon_data = include_bytes!("../icon.ico");
    let image = image::load_from_memory(icon_data)
        .expect("Failed to load icon from memory")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

fn main() {
    let config_path = get_config_path();
    let dirs = load_dirs(config_path.to_str().unwrap());

    println!("Loaded len {} directories", dirs.len());

    if dirs.is_empty() {
        eprintln!("No directories found in config.txt");
    }

    let images = load_images_from_dirs(&dirs);

    println!("Loaded {} images", images.len());
    let state = Arc::new(Mutex::new(AppState::new(images)));

    // 启动时立即尝试设置第一张壁纸
    {
        let s = state.lock().unwrap();
        apply_wallpaper(&s);
    }

    // 打断背景线程的信号量
    let (worker_tx, worker_rx) = mpsc::channel::<()>();

    // 自动循环线程
    {
        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            loop {
                // 读取当前的延时配置
                let timeout = {
                    let s = state_clone.lock().unwrap();
                    s.loop_time.as_duration()
                };

                if let Some(dur) = timeout {
                    match worker_rx.recv_timeout(dur) {
                        Ok(_) => {
                            // 收到设置变更信号，跳过等待立刻重新计算 dur
                            // 消耗积压的所有信号，确保定时器重新起步
                            while let Ok(_) = worker_rx.try_recv() {}
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            let mut s = state_clone.lock().unwrap();
                            s.next();
                            apply_wallpaper(&s);
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                } else {
                    // 如果关闭循坏，一直阻塞等待设置变更信号
                    match worker_rx.recv() {
                        Ok(_) => {
                            while let Ok(_) = worker_rx.try_recv() {}
                        },
                        Err(_) => break,
                    }
                }
            }
        });
    }

    // ===== 构建菜单 =====
    let menu = Menu::new();

    // 1) 循环时间子菜单
    let loop_time_menu = Submenu::new("循环时间", true);
    let time_5s = CheckMenuItem::new("5秒", true, true, None);
    let time_10s = CheckMenuItem::new("10秒", true, false, None);
    let time_1m = CheckMenuItem::new("1分钟", true, false, None);
    let time_1d = CheckMenuItem::new("1天", true, false, None);
    let time_off = CheckMenuItem::new("关闭", true, false, None);

    let time_items: Vec<&dyn IsMenuItem> = vec![&time_5s, &time_10s, &time_1m, &time_1d, &time_off];
    let _ = loop_time_menu.append_items(&time_items);

    // 2) 播放顺序子菜单
    let play_order_menu = Submenu::new("播放顺序", true);
    let order_seq = CheckMenuItem::new("顺序", true, true, None);
    let order_rand = CheckMenuItem::new("随机", true, false, None);

    let order_items: Vec<&dyn IsMenuItem> = vec![&order_seq, &order_rand];
    let _ = play_order_menu.append_items(&order_items);

    // 3) 其他基础菜单
    let refresh_btn = MenuItem::new("刷新", true, None);
    let next_btn = MenuItem::new("下一张", true, None);
    let prev_btn = MenuItem::new("上一张", true, None);
    let explorer_btn = MenuItem::new("打开目录", true, None);
    let quit_btn = MenuItem::new("退出", true, None);

    let mut dir = dirs.get(0).cloned().unwrap_or_default();

    let _ = menu.append_items(&[
        &loop_time_menu,
        &play_order_menu,
        &PredefinedMenuItem::separator(),
        &refresh_btn,
        &prev_btn,
        &next_btn,
        &PredefinedMenuItem::separator(),
        &explorer_btn,
        &PredefinedMenuItem::separator(),
        &quit_btn,
    ]);

    let icon = load_icon();
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Wallpaper Switcher")
        .with_icon(icon)
        .build()
        .unwrap();

    let menu_channel = MenuEvent::receiver();

    // 维持系统事件循环接收（否则托盘不响应且不绘制菜单）
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, MSG, TranslateMessage,
        };
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // 每捕获一条系统消息，检查所有积压的菜单事件
            while let Ok(event) = menu_channel.try_recv() {
                let id = &event.id;

                if id == time_5s.id() {
                    let mut s = state.lock().unwrap();
                    s.loop_time = LoopTime::Sec5;
                    time_5s.set_checked(true);
                    time_10s.set_checked(false);
                    time_1m.set_checked(false);
                    time_1d.set_checked(false);
                    time_off.set_checked(false);
                    let _ = worker_tx.send(());
                } else if id == time_10s.id() {
                    let mut s = state.lock().unwrap();
                    s.loop_time = LoopTime::Sec10;
                    time_5s.set_checked(false);
                    time_10s.set_checked(true);
                    time_1m.set_checked(false);
                    time_1d.set_checked(false);
                    time_off.set_checked(false);
                    let _ = worker_tx.send(());
                } else if id == time_1m.id() {
                    let mut s = state.lock().unwrap();
                    s.loop_time = LoopTime::Min1;
                    time_5s.set_checked(false);
                    time_10s.set_checked(false);
                    time_1m.set_checked(true);
                    time_1d.set_checked(false);
                    time_off.set_checked(false);
                    let _ = worker_tx.send(());
                } else if id == time_1d.id() {
                    let mut s = state.lock().unwrap();
                    s.loop_time = LoopTime::Day1;
                    time_5s.set_checked(false);
                    time_10s.set_checked(false);
                    time_1m.set_checked(false);
                    time_1d.set_checked(true);
                    time_off.set_checked(false);
                    let _ = worker_tx.send(());
                } else if id == time_off.id() {
                    let mut s = state.lock().unwrap();
                    s.loop_time = LoopTime::Off;
                    time_5s.set_checked(false);
                    time_10s.set_checked(false);
                    time_1m.set_checked(false);
                    time_1d.set_checked(false);
                    time_off.set_checked(true);
                    let _ = worker_tx.send(());
                } else if id == order_seq.id() {
                    let mut s = state.lock().unwrap();
                    s.play_order = PlayOrder::Sequential;
                    order_seq.set_checked(true);
                    order_rand.set_checked(false);
                    let _ = worker_tx.send(());
                } else if id == order_rand.id() {
                    let mut s = state.lock().unwrap();
                    s.play_order = PlayOrder::Random;
                    order_seq.set_checked(false);
                    order_rand.set_checked(true);
                    let _ = worker_tx.send(());
                } else if id == refresh_btn.id() {
                    let config_path = get_config_path();
                    let abs_path = std::fs::canonicalize(&config_path).unwrap_or(config_path.clone());
                    println!("--- 刷新中 ---");
                    println!("使用配置文件: {:?}", abs_path);
                    
                    let new_dirs = load_dirs(config_path.to_str().unwrap());
                    println!("加载目录库: {:?}", new_dirs);
                    
                    let new_images = load_images_from_dirs(&new_dirs);
                    println!("扫描到图片总数: {}", new_images.len());
                    
                    if let Some(first_dir) = new_dirs.get(0) {
                        dir = first_dir.clone();
                    }

                    let mut s = state.lock().unwrap();
                    if new_images.is_empty() {
                        println!("警告: 未找到任何图片，保持原有列表。");
                    } else {
                        // 尝试保留当前的壁纸位置
                        let current_path = s.current_path().cloned();
                        s.images = new_images;
                        
                        if let Some(path) = current_path {
                            if let Some(new_idx) = s.images.iter().position(|p| p == &path) {
                                s.current = new_idx;
                                println!("刷新完成，已保留当前壁纸位置。");
                            } else {
                                s.current = 0;
                                apply_wallpaper(&s);
                                println!("刷新完成，原壁纸已不在列表中，重置到第一张。");
                            }
                        } else {
                            s.current = 0;
                            apply_wallpaper(&s);
                        }
                    }
                    // 发送信号让后台线程根据“当前保留的配置”重新开始计时或继续休眠
                    let _ = worker_tx.send(());
                    println!("---------------");
                } else if id == next_btn.id() {
                    let mut s = state.lock().unwrap();
                    s.next();
                    apply_wallpaper(&s);
                    // 点击下一张相当于重置了当前进度，打断 sleep
                    let _ = worker_tx.send(());
                } else if id == prev_btn.id() {
                    let mut s = state.lock().unwrap();
                    s.prev();
                    apply_wallpaper(&s);
                    let _ = worker_tx.send(());
                } else if id == explorer_btn.id() {
                    if !dir.is_empty() {
                        let _ = Command::new("explorer").arg(&dir).spawn();
                    }
                } else if id == quit_btn.id() {
                    std::process::exit(0);
                }
            }
        }
    }
}
