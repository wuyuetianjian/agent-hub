use crate::window_state::Bounds;
use crate::{model::*, runtime::Runtime};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
#[derive(Default)]
struct NativeState {
    tray_available: AtomicBool,
    view: Mutex<Option<(String, bool)>>,
}
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, LogicalPosition, LogicalSize, Manager, State,
};
use tauri_plugin_autostart::ManagerExt;
type CommandResult<T> = Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[tauri::command]
async fn get_dashboard(runtime: State<'_, Arc<Runtime>>) -> CommandResult<Dashboard> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.dashboard())
        .await
        .map_err(error)?
        .map_err(error)
}

#[tauri::command]
async fn refresh_profile(
    runtime: State<'_, Arc<Runtime>>,
    profile_id: String,
) -> CommandResult<QuotaSnapshot> {
    runtime.refresh(&profile_id).await.map_err(error)
}
#[tauri::command]
fn save_profile(runtime: State<'_, Arc<Runtime>>, profile: Profile) -> CommandResult<()> {
    runtime.save_profile(profile).map_err(error)
}
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    settings: Settings,
) -> CommandResult<()> {
    crate::config::validate_settings(&settings).map_err(error)?;
    if settings.window_mode == "island" && !cfg!(target_os = "macos") {
        return Err("Dynamic Island mode is available on macOS".into());
    }
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(settings.always_on_top)
            .map_err(error)?;
    }
    let previous = runtime.settings.read().unwrap().start_at_login;
    if settings.start_at_login != previous {
        if settings.start_at_login {
            app.autolaunch().enable().map_err(error)?;
        } else {
            app.autolaunch().disable().map_err(error)?;
        }
    }
    runtime.save_settings(settings).map_err(error)
}
#[tauri::command]
async fn start_session(
    runtime: State<'_, Arc<Runtime>>,
    profile_id: String,
    cwd: String,
    mode: String,
) -> CommandResult<SessionInfo> {
    let (p, provider) = runtime.profile(&profile_id).map_err(error)?;
    let settings = runtime.settings.read().unwrap().clone();
    let bridge = runtime.bridge_url.read().unwrap().clone();
    runtime
        .sessions
        .start(&p, &provider, &cwd, &mode, &settings, &bridge)
        .await
        .map_err(error)
}
#[tauri::command]
fn write_session(
    runtime: State<'_, Arc<Runtime>>,
    session_id: String,
    data: String,
) -> CommandResult<()> {
    runtime.sessions.write(&session_id, &data).map_err(error)
}
#[tauri::command]
async fn prompt_session(
    runtime: State<'_, Arc<Runtime>>,
    session_id: String,
    text: String,
) -> CommandResult<()> {
    runtime
        .sessions
        .prompt(&session_id, &text)
        .await
        .map_err(error)
}
#[tauri::command]
fn resize_session(
    runtime: State<'_, Arc<Runtime>>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> CommandResult<()> {
    runtime
        .sessions
        .resize(&session_id, cols, rows)
        .map_err(error)
}
#[tauri::command]
fn session_replay(
    runtime: State<'_, Arc<Runtime>>,
    session_id: String,
) -> CommandResult<serde_json::Value> {
    runtime.sessions.replay(&session_id).map_err(error)
}
#[tauri::command]
fn stop_session(runtime: State<'_, Arc<Runtime>>, session_id: String) -> CommandResult<()> {
    runtime.sessions.stop(&session_id).map_err(error)
}
#[tauri::command]
fn respond(
    runtime: State<'_, Arc<Runtime>>,
    request_id: String,
    reply: Reply,
) -> CommandResult<()> {
    runtime.broker.respond(&request_id, reply).map_err(error)?;
    runtime.emit("dashboard");
    Ok(())
}
fn save_geometry(app: &tauri::AppHandle, runtime: &Runtime) {
    let native = app.state::<NativeState>();
    let view = native.view.lock().unwrap().clone();
    if let Some((view, transient)) =
        view.filter(|(view, transient)| !transient || view.starts_with("dock"))
    {
        let _ = transient;
        let view = if view.starts_with("dock") {
            "dock".to_string()
        } else {
            view
        };
        if let Some(window) = app.get_webview_window("main") {
            if let (Ok(position), Ok(size), Ok(scale)) = (
                window.inner_position(),
                window.inner_size(),
                window.scale_factor(),
            ) {
                let _ = runtime.store.save_window_bounds(
                    &view,
                    Bounds {
                        x: position.x as f64 / scale,
                        y: position.y as f64 / scale,
                        width: size.width as f64 / scale,
                        height: size.height as f64 / scale,
                    },
                );
            }
        }
    }
}
fn layout(app: &tauri::AppHandle, runtime: &Runtime, view: &str) -> CommandResult<()> {
    if !["widget", "workspace", "detail", "dock", "dockDetail"].contains(&view) {
        return Err("Unknown window view".into());
    }
    let window = app.get_webview_window("main").ok_or("Window unavailable")?;
    save_geometry(app, runtime);
    let s = runtime.settings.read().unwrap().clone();
    let view = if view == "widget" && s.window_mode == "floating" {
        "dock"
    } else {
        view
    };
    if view == "dock" || view == "dockDetail" {
        let sessions = runtime.sessions.list();
        let pending = runtime.broker.list();
        let pending_profiles: std::collections::HashSet<_> = sessions
            .iter()
            .filter(|session| pending.iter().any(|r| r.session_id == session.id))
            .map(|s| s.profile_id.as_str())
            .collect();
        let count = runtime
            .store
            .profiles()
            .map_err(error)?
            .iter()
            .filter(|p| {
                (p.enabled && s.visible_providers.contains(&p.provider_id))
                    || pending_profiles.contains(p.id.as_str())
            })
            .count();
        let rail_width = s.dock_size + 2.0 * s.dock_padding;
        let height = (2.0 * s.dock_padding
            + count as f64 * s.dock_size
            + count.saturating_sub(1) as f64 * s.dock_gap
            + s.dock_chrome_height)
            .max(s.dock_popup_height);
        let width = rail_width
            + if view == "dockDetail" {
                s.dock_gap + s.dock_panel_width
            } else {
                0.0
            };
        if let Some(m) = window
            .current_monitor()
            .ok()
            .flatten()
            .or(window.primary_monitor().ok().flatten())
        {
            let scale = m.scale_factor();
            let display = Bounds {
                x: m.position().x as f64 / scale,
                y: m.position().y as f64 / scale,
                width: m.size().width as f64 / scale,
                height: m.size().height as f64 / scale,
            };
            let bounds = crate::window_state::dock_bounds_at(
                display,
                width,
                height,
                &s.dock_side,
                s.dock_edge_inset,
                runtime
                    .store
                    .window_bounds("dock")
                    .ok()
                    .flatten()
                    .map(|b| b.y),
            );
            let _ = window.set_min_size(Some(LogicalSize::new(64.0, 80.0)));
            window
                .set_size(LogicalSize::new(bounds.width, bounds.height))
                .map_err(error)?;
            let _ = window.set_position(LogicalPosition::new(bounds.x, bounds.y));
        } else {
            window
                .set_size(LogicalSize::new(width, height))
                .map_err(error)?;
        }
        let _ = window.set_shadow(false);
        let _ = window.set_resizable(false);
        let _ = window.set_always_on_top(s.always_on_top);
        *app.state::<NativeState>().view.lock().unwrap() = Some((view.into(), true));
        window.show().map_err(error)?;
        return Ok(());
    }
    let _ = window.set_shadow(true);
    let _ = window.set_min_size(Some(LogicalSize::new(360.0, 80.0)));
    let _ = window.set_resizable(true);
    let island = s.window_mode == "island" && view != "workspace";
    let desired = if view == "workspace" {
        (s.expanded_width, s.expanded_height)
    } else if island {
        (
            s.island_width,
            if view == "detail" {
                s.window_height
            } else {
                s.island_height
            },
        )
    } else {
        (
            s.window_width,
            if s.compact { 230.0 } else { s.window_height },
        )
    };
    let monitors = window.available_monitors().unwrap_or_default();
    let displays: Vec<_> = monitors
        .iter()
        .map(|m| {
            let scale = m.scale_factor();
            Bounds {
                x: m.position().x as f64 / scale,
                y: m.position().y as f64 / scale,
                width: m.size().width as f64 / scale,
                height: m.size().height as f64 / scale,
            }
        })
        .collect();
    let saved = runtime.store.window_bounds(view).map_err(error)?;
    let bounds = crate::window_state::restore(saved, desired, &displays);
    window
        .set_size(LogicalSize::new(bounds.width, bounds.height))
        .map_err(error)?;
    if island {
        if let Some(m) = window
            .current_monitor()
            .ok()
            .flatten()
            .or(window.primary_monitor().ok().flatten())
        {
            let scale = m.scale_factor();
            let x = m.position().x as f64 / scale
                + (m.size().width as f64 / scale - bounds.width) / 2.0;
            let y = m.position().y as f64 / scale;
            window
                .set_position(LogicalPosition::new(x, y))
                .map_err(error)?;
        }
    } else if !displays.is_empty() {
        let _ = window.set_position(LogicalPosition::new(bounds.x, bounds.y));
    }
    let _ = window.set_always_on_top(s.always_on_top);
    *app.state::<NativeState>().view.lock().unwrap() = Some((view.into(), island));
    window.show().map_err(error)?;
    Ok(())
}

#[tauri::command]
fn set_window_view(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    view: String,
) -> CommandResult<()> {
    layout(&app, &runtime, &view)
}
#[tauri::command]
fn hide_window(app: tauri::AppHandle, runtime: State<'_, Arc<Runtime>>) -> CommandResult<()> {
    if !app
        .state::<NativeState>()
        .tray_available
        .load(Ordering::Relaxed)
    {
        return Err("System tray is unavailable; keep this window open".into());
    }
    save_geometry(&app, &runtime);
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(error)?;
    }
    Ok(())
}
#[tauri::command]
fn quit_app(app: tauri::AppHandle, runtime: State<'_, Arc<Runtime>>) {
    save_geometry(&app, &runtime);
    runtime.sessions.stop_all();
    app.exit(0);
}

fn startup_trace(phase: &str) {
    use std::io::Write;
    if let Some(path) = std::env::var_os("AGENT_ORBIT_STARTUP_TRACE") {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{phase}");
        }
    }
}
pub fn run() {
    startup_trace("initializing local runtime");
    let runtime = Runtime::new().expect("Unable to initialize local data store");
    let rt = runtime.clone();
    startup_trace("building native application");
    let app = tauri::Builder::default()
        .manage(runtime)
        .manage(NativeState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_dashboard,
            refresh_profile,
            save_profile,
            save_settings,
            start_session,
            write_session,
            prompt_session,
            resize_session,
            session_replay,
            stop_session,
            respond,
            set_window_view,
            hide_window,
            quit_app
        ])
        .setup(move |app| {
            startup_trace("native setup callback");
            let handle = app.handle().clone();
            let events_handle = handle.clone();
            let mut events = rt.events.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(event) => {
                            let _ = events_handle.emit("orbit:event", event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            });
            let background = rt.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::bridge::serve(background.clone()).await {
                    eprintln!("Local hook bridge unavailable: {e}");
                }
                background.poll().await;
            });
            let show = MenuItem::with_id(app, "show", "Show Agent Orbit", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip(&rt.config.app_name)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let runtime = app.state::<Arc<Runtime>>();
                        let view = app
                            .state::<NativeState>()
                            .view
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|(v, _)| v.clone())
                            .unwrap_or_else(|| "widget".into());
                        let _ = layout(app, &runtime, &view);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        save_geometry(app, &app.state::<Arc<Runtime>>());
                        app.state::<Arc<Runtime>>().sessions.stop_all();
                        app.exit(0);
                    }
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            // A desktop without tray support still retains the normal window.
            app.state::<NativeState>()
                .tray_available
                .store(tray.build(app).is_ok(), Ordering::Relaxed);
            startup_trace("creating initial window layout");
            layout(&handle, &rt, "widget").map_err(anyhow::Error::msg)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Unable to start Agent Orbit");
    startup_trace("starting native event loop");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            save_geometry(app, &app.state::<Arc<Runtime>>());
            app.state::<Arc<Runtime>>().sessions.stop_all();
        }
    });
}
