// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod consts;
mod errors;
mod events;
mod logger;
mod maa_cli;
mod status;
mod utils;
mod version;

use std::env;
use pci_info::{enumerators, enumerators::PciEnumerator};

use tauri::{
    api::notification::Notification, CustomMenuItem, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem,
};

use events::{
    copilot, delete_user_config, get_cli_config, get_item_index, get_fight_stages, get_current_sidestory, 
    get_user_configs, ignore_maa_cli_update, init_process, maa_cli_update_process, one_key, save_cli_config,
    save_core_config, save_task_config, stop,
};

fn main() {
    let maa_bo_dir = maa_cli::dir();
    if !maa_bo_dir.exists() {
        utils::make_dir_exist(&maa_bo_dir).unwrap();
    }
    logger::init_logger(&maa_bo_dir, &"maabo");
    log::info!("MaaBo 版本:{}", env!("CARGO_PKG_VERSION"));
    log::info!("tauri 版本:{}", tauri::VERSION);
    log::info!(
        "webview 版本:{}",
        tauri::webview_version().unwrap_or_else(|error| {
            log::info!("获取webview版本失败,请确认是否安装\n{}", error.to_string());
            panic!("获取webview版本失败,请确认是否安装\n{}", error.to_string());
        })
    );

    if env::consts::OS == "linux" {
        // 使用pci-info库来枚举 PCI 设备
        let enumerator = enumerators::LinuxProcFsPciEnumerator::Fastest;
        let pci_info = enumerator.enumerate_pci().unwrap();

        for device in pci_info.iter().filter_map(Result::ok) {
            // Nvidia的供应商ID通常为0x10DE
            if device.vendor_id() == 0x10DE {
                // 不想引入额外库的话直接对Linux全体禁用也行。
                env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                log::info!("检测到NVIDIA GPU, 启动兼容模式");
                break
            }
        }
    }

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("tips".to_string(), "给我一个提示"))
        .add_item(CustomMenuItem::new("visible".to_string(), "最小化到托盘"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit".to_string(), "退出"));

    let tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_process,
            get_cli_config,
            get_user_configs,
            save_cli_config,
            save_core_config,
            save_task_config,
            delete_user_config,
            one_key,
            stop,
            ignore_maa_cli_update,
            maa_cli_update_process,
            get_item_index,
            get_fight_stages,
            get_current_sidestory,
            copilot
        ])
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            // SystemTrayEvent::LeftClick {
            //     position: _,
            //     size: _,
            //     ..
            // } => {
            //     println!("system tray received a left click");
            // }
            // SystemTrayEvent::RightClick {
            //     position: _,
            //     size: _,
            //     ..
            // } => {
            //     println!("system tray received a right click");
            // }
            SystemTrayEvent::DoubleClick {
                position: _,
                size: _,
                ..
            } => {
                let window = app.get_window("main").unwrap();
                if !window.is_visible().unwrap() {
                    window.show().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    status::kill_all();
                    std::process::exit(0);
                }
                "visible" => {
                    let item_handle = app.tray_handle().get_item(&id);

                    let window = app.get_window("main").unwrap();
                    if window.is_visible().unwrap() {
                        item_handle.set_title("显示").unwrap();
                        window.hide().unwrap();
                    } else {
                        window.show().unwrap();
                        item_handle.set_title("最小化到托盘").unwrap();
                    }
                }
                "tips" => {
                    let handle = app.app_handle();
                    Notification::new(&handle.config().tauri.bundle.identifier)
                        .title("我没想过真的有人会按这个")
                        .body("真的")
                        .notify(&handle)
                        .unwrap();
                }
                _ => {}
            },
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(move |_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } => {
            status::kill_all();
        }
        _ => {}
    });
}
