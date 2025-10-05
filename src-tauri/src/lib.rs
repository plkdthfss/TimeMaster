mod db;

use crate::db::{Db, IdPayload, NewTask, Task, UpdateTask};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewWindow};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_tasks(db: State<Db>, status: Option<String>) -> Result<Vec<Task>, String> {
    db.list_tasks(status).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_task(db: State<Db>, payload: NewTask) -> Result<Task, String> {
    db.create_task(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_task(db: State<Db>, payload: UpdateTask) -> Result<Task, String> {
    db.update_task(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(db: State<Db>, payload: IdPayload) -> Result<(), String> {
    db.delete_task(&payload.id).map_err(|e| e.to_string())
}

#[tauri::command]
fn increase_task_progress(db: State<Db>, payload: IdPayload) -> Result<Task, String> {
    db.increment_progress(&payload.id).map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_task(db: State<Db>, payload: IdPayload) -> Result<Task, String> {
    db.archive_task(&payload.id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reopen_task(db: State<Db>, payload: IdPayload) -> Result<Task, String> {
    db.reopen_task(&payload.id).map_err(|e| e.to_string())
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum DockSide {
    Left,
    Right,
}

impl DockSide {
    fn as_str(&self) -> &'static str {
        match self {
            DockSide::Left => "left",
            DockSide::Right => "right",
        }
    }
}

const MIN_PANEL_WIDTH: f64 = 360.0;
const MAX_PANEL_WIDTH: f64 = 520.0;

fn configure_main_window(window: &WebviewWindow) -> tauri::Result<()> {
    if let Some(monitor) = window.current_monitor()? {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let logical_width = size.width as f64 / scale;
        let logical_height = size.height as f64 / scale;
        let target_width = (logical_width * 0.32).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        let taskbar_height = 48.0;

        window.set_decorations(false)?;
        window.set_always_on_top(false)?;
        window.set_resizable(false)?;
        window.set_size(LogicalSize::new(target_width, logical_height - taskbar_height))?;

        let position = monitor.position();
        let offset_x = position.x as f64 / scale;
        let offset_y = position.y as f64 / scale;
        window.set_position(LogicalPosition::new(
            offset_x + logical_width - target_width,
            offset_y,
        ))?;
    }

    Ok(())
}

fn position_main_window(main: &WebviewWindow, side: DockSide) -> Result<(), String> {
    let monitor = main
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "monitor information unavailable".to_string())?;

    let scale = monitor.scale_factor();
    let size = monitor.size();
    let logical_width = size.width as f64 / scale;
    let logical_height = size.height as f64 / scale;
    let target_width = (logical_width * 0.32).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);

    let position = monitor.position();
    let offset_x = position.x as f64 / scale;
    let offset_y = position.y as f64 / scale;

    let x = match side {
        DockSide::Left => offset_x,
        DockSide::Right => offset_x + logical_width - target_width,
    };

    main
        .set_size(LogicalSize::new(target_width, logical_height))
        .map_err(|e| e.to_string())?;
    main
        .set_position(LogicalPosition::new(x, offset_y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reveal_panel(app: AppHandle, side: DockSide) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;

    position_main_window(&main, side)?;
    main.show().map_err(|e| e.to_string())?;
    let _ = main.set_focus();
    let _ = main.emit("time-master::dock", side.as_str());

    Ok(())
}

#[tauri::command]
fn conceal_panel(app: AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let _ = main;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            list_tasks,
            create_task,
            update_task,
            delete_task,
            increase_task_progress,
            archive_task,
            reopen_task,
            reveal_panel,
            conceal_panel
        ])
        .setup(|app| {
            let app_handle = app.handle();
            let db = Db::init(&app_handle)?;
            app.manage(db);

            let main_window = app_handle
                .get_webview_window("main")
                .expect("main window missing");
            configure_main_window(&main_window)?;

            let last_trigger = Arc::new(Mutex::new(Instant::now()));
            let global_shortcut = app_handle.global_shortcut();
            let last_trigger_clone = Arc::clone(&last_trigger);

            global_shortcut.on_shortcut("Ctrl+Q", move |handle, _shortcut, _| {
                let mut last = last_trigger_clone.lock().unwrap();
                let now = Instant::now();
                if now.duration_since(*last) < Duration::from_millis(200) {
                    return;
                }
                *last = now;

                if let Some(window) = handle.get_webview_window("main") {
                    match window.is_visible() {
                        Ok(true) => {
                            let _ = window.hide();
                        }
                        Ok(false) => {
                            if position_main_window(&window, DockSide::Right).is_ok() {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        Err(err) => {
                            eprintln!("failed to read window visibility: {err}");
                        }
                    }
                }
            })?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


