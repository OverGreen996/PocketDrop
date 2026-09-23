#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;
#[cfg(test)]
mod files;
#[cfg(windows)]
mod native_frame;
mod phone;

use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[tauri::command]
async fn open_backdrop(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("backdrop") {
        return window.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(
        &app,
        "backdrop",
        tauri::WebviewUrl::App("backdrop.html".into()),
    )
    .title("PocketDrop M0 外部背景測試板")
    .inner_size(1280.0, 900.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn set_material(window: tauri::WebviewWindow, mode: String) -> Result<String, String> {
    let (send, recv) = tokio::sync::oneshot::channel();
    let target = window.clone();
    let selected = mode.clone();
    window
        .run_on_main_thread(move || {
            let result = match selected.as_str() {
                "acrylic" => native_frame::material(&target, true),
                "off" => native_frame::material(&target, false),
                _ => Err("Unknown material".into()),
            };
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
    recv.await.map_err(|_| "材質初始化中斷")??;
    Ok(if mode == "acrylic" {
        "原生 Acrylic 已啟用"
    } else {
        "原生模糊已關閉"
    }
    .into())
}

#[tauri::command]
async fn initialize_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .set_zoom(1.0 / window.scale_factor().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    window
        .set_size(tauri::LogicalSize::new(440.0, 740.0))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn sync_window_shape(
    window: tauri::WebviewWindow,
    viewport_width: f64,
) -> Result<(), String> {
    let (send, recv) = tokio::sync::oneshot::channel();
    let target = window.clone();
    window
        .run_on_main_thread(move || {
            #[cfg(windows)]
            let result = native_frame::round(&target, Some(viewport_width));
            #[cfg(not(windows))]
            let result = Ok(());
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
    recv.await.map_err(|_| "圓角更新中斷")?
}

fn handle_window_drop<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::DragDrop(drop) = event {
        handle_drop(app, drop);
    }
}

fn handle_drop<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: &tauri::DragDropEvent) {
    match event {
        tauri::DragDropEvent::Enter { .. } | tauri::DragDropEvent::Over { .. } => {
            let _ = app.emit("drop-hover", true);
        }
        tauri::DragDropEvent::Leave => {
            let _ = app.emit("drop-hover", false);
        }
        tauri::DragDropEvent::Drop { paths, .. } => {
            let _ = app.emit("drop-hover", false);
            let paths = paths.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                let mut accepted = 0;
                let mut error = if paths.len() > 100 {
                    Some("每次最多加入 100 個檔案".to_string())
                } else {
                    None
                };
                match phone::current(&app.state::<phone::Phone>()) {
                    Ok(core) => {
                        for path in paths.into_iter().take(100) {
                            match core.add_file(path, "電腦") {
                                Ok(()) => accepted += 1,
                                Err(e) => error = Some(e),
                            }
                        }
                    }
                    Err(e) => error = Some(e),
                }
                eprintln!(
                    "{{\"event\":\"native_drop\",\"accepted\":{},\"failed\":{}}}",
                    accepted,
                    error.is_some()
                );
                let _ = app.emit(
                    "phone-drop",
                    serde_json::json!({"accepted":accepted,"error":error}),
                );
            });
        }
        _ => {}
    }
}

fn main() {
    if let Some(dir) = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|a| a[0] == "--phone-smoke")
        .map(|a| std::path::PathBuf::from(&a[1]))
    {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let core = phone::Core::open(&dir, dir.join("inbox")).unwrap();
            let ip = if std::env::args().any(|a| a == "--lan-interop") {
                if_addrs::get_if_addrs()
                    .unwrap()
                    .into_iter()
                    .find_map(|i| match i.ip() {
                        std::net::IpAddr::V4(ip) if phone::local_ip(ip) => Some(ip),
                        _ => None,
                    })
                    .expect("LAN interface")
            } else {
                std::net::Ipv4Addr::LOCALHOST
            };
            phone::serve(core.clone(), ip, true).await.unwrap();
            let invite = core.new_invite().unwrap();
            std::fs::write(
                dir.join("invite.json"),
                serde_json::to_vec(&invite).unwrap(),
            )
            .unwrap();
            std::fs::write(dir.join("bootstrap.json"), invite.qr).unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(180)).await;
        });
        return;
    }
    let app = tauri::Builder::default()
        .manage(phone::Phone(Mutex::new(None)))
        .manage(discovery::Discovery(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            initialize_window,
            sync_window_shape,
            set_material,
            open_backdrop,
            discovery::interfaces,
            discovery::start_discovery,
            discovery::stop_discovery,
            phone::phone_start,
            phone::phone_snapshot,
            phone::phone_invite,
            phone::phone_text,
            phone::phone_revoke,
            phone::phone_remove,
            phone::peer::nearby_rooms,
            phone::peer::join_qr,
            phone::peer::join_code,
            phone::peer::room_mode,
            phone::peer::leave_room,
            phone::peer::download_file,
            phone::peer::cancel_download,
            phone::peer::show_downloads
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .ok_or("missing main window")?;
            #[cfg(windows)]
            native_frame::install(&window)?;
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                // WindowContent webviews route native drops to WindowEvent,
                // not Builder::on_webview_event (see tauri-runtime-wry).
                handle_window_drop(&handle, event);
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                    handle.exit(0);
                }
                if matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. }) {
                    let next = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        if let Some(w) = next.get_webview_window("main") {
                            #[cfg(windows)]
                            if native_frame::round(&w, None).is_err() {
                                let _ = next.emit("m0-error", "原生圓角更新失敗");
                            }
                        }
                    });
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Unable to start PocketDrop M0");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Ok(mut state) = app.state::<discovery::Discovery>().0.lock() {
                state.take();
            }
        }
    });
}

#[cfg(test)]
mod drop_regression {
    use super::*;
    use tauri::Listener;
    #[test]
    fn window_drop_publishes_multiple_files_and_rejects_directory() {
        let root = std::env::temp_dir().join(format!("pd-drop-{}", uuid::Uuid::new_v4()));
        let core = phone::Core::open(&root, root.join("inbox")).unwrap();
        let app = tauri::test::mock_builder()
            .manage(phone::Phone(Mutex::new(Some(core.clone()))))
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let (send, recv) = std::sync::mpsc::channel();
        app.listen("phone-drop", move |event| {
            let _ = send.send(event.payload().to_string());
        });
        let a = root.join("模型.stl");
        let b = root.join("notes.txt");
        std::fs::write(&a, b"solid test").unwrap();
        std::fs::write(&b, b"notes").unwrap();
        handle_window_drop(
            app.handle(),
            &tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop {
                paths: vec![a, b, root.clone()],
                position: tauri::PhysicalPosition::new(20.0, 20.0),
            }),
        );
        let event: serde_json::Value = serde_json::from_str(
            &recv
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(event["accepted"], 2);
        assert!(event["error"].as_str().unwrap().contains("ZIP"));
        let state = core.snapshot().unwrap();
        assert_eq!(state.files.len(), 2);
        assert!(state.files.iter().all(|f| f.available));
        assert!(!serde_json::to_string(&state)
            .unwrap()
            .contains("local_path"));
        // Wait for the background hash jobs before removing this test-owned directory.
        for _ in 0..100 {
            if core
                .snapshot()
                .unwrap()
                .files
                .iter()
                .all(|f| f.sha256.is_some())
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        app.state::<phone::Phone>().0.lock().unwrap().take();
        for _ in 0..100 {
            if std::sync::Arc::strong_count(&core) == 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(state);
        drop(app);
        drop(core);
        std::fs::remove_dir_all(root).unwrap();
    }
}
