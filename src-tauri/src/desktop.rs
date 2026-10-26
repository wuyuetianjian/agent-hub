use crate::window_state::Bounds;
use crate::{model::*, runtime::Runtime};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
#[derive(Default)]
struct NativeState {
    tray_available: AtomicBool,
    dragging: AtomicBool,
    drag_origin: Mutex<Option<Bounds>>,
    panel: Mutex<Option<PanelTarget>>,
    view: Mutex<Option<(String, bool)>>,
    announced_approvals: Mutex<std::collections::HashSet<String>>,
    completion_ready: AtomicBool,
    shown_completion: Mutex<Option<String>>,
    placed_dock_side: Mutex<Option<String>>,
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PanelTarget {
    profile_id: String,
    anchor: f64,
    pinned: bool,
    height: f64,
}
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
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

// Completion notifications have their own window so dock hover dismissal never
// consumes them. The queue remains in Sessions across webview reloads.
#[tauri::command]
fn get_task_completions(app: tauri::AppHandle) -> CommandResult<Vec<TaskCompletion>> {
    app.state::<NativeState>()
        .completion_ready
        .store(true, Ordering::Relaxed);
    sync_completion_popup(&app)?;
    Ok(app.state::<Arc<Runtime>>().sessions.list_completions())
}

#[tauri::command]
fn dismiss_task_completion(app: tauri::AppHandle, id: String) -> CommandResult<()> {
    app.state::<Arc<Runtime>>().sessions.dismiss_completion(&id);
    sync_completion_popup(&app)
}

fn sync_completion_popup(app: &tauri::AppHandle) -> CommandResult<()> {
    // Tauri window getters wait for the UI thread. Serialize the entire operation
    // there, never with a mutex that UI commands might block on from another thread.
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Err(error) = sync_completion_popup_on_main(&handle) {
            eprintln!("Completion popup unavailable: {error}");
        }
    })
    .map_err(error)
}

fn sync_completion_popup_on_main(app: &tauri::AppHandle) -> CommandResult<()> {
    let native = app.state::<NativeState>();
    let runtime = app.state::<Arc<Runtime>>();
    let completions = runtime.sessions.list_completions();
    let has_approvals = runtime
        .broker
        .list()
        .iter()
        .any(|request| request.expires_at > chrono::Utc::now().timestamp());
    let Some(window) = app.get_webview_window("task-completion") else {
        return Ok(());
    };
    if !crate::completion_window::completion_visible(
        native.completion_ready.load(Ordering::Relaxed),
        !completions.is_empty(),
        has_approvals,
    ) {
        *native.shown_completion.lock().unwrap() = None;
        return window.hide().map_err(error);
    }
    let dock = app.get_webview_window("main").ok_or("Dock unavailable")?;
    if let Some(monitor) = dock
        .current_monitor()
        .map_err(error)?
        .or(window.primary_monitor().map_err(error)?)
    {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let display = Bounds {
            x: area.position.x as f64 / scale,
            y: area.position.y as f64 / scale,
            width: area.size.width as f64 / scale,
            height: area.size.height as f64 / scale,
        };
        let dock_scale = dock.scale_factor().map_err(error)?;
        let position = dock.inner_position().map_err(error)?;
        let size = dock.inner_size().map_err(error)?;
        let settings = runtime.settings.read().unwrap();
        let bounds = crate::window_state::panel_bounds(
            display,
            Bounds {
                x: position.x as f64 / dock_scale,
                y: position.y as f64 / dock_scale,
                width: size.width as f64 / dock_scale,
                height: size.height as f64 / dock_scale,
            },
            360.0,
            210.0,
            &settings.dock_side,
            settings.dock_gap,
            105.0,
        );
        window
            .set_size(LogicalSize::new(bounds.width, bounds.height))
            .map_err(error)?;
        window
            .set_position(LogicalPosition::new(bounds.x, bounds.y))
            .map_err(error)?;
    }
    *native.shown_completion.lock().unwrap() = completions.first().map(|item| item.id.clone());
    if !window.is_visible().map_err(error)? {
        // Keep editor focus on automatic arrival; explicit button clicks may focus.
        window.set_focusable(false).map_err(error)?;
        let shown = window.show().map_err(error);
        let restored = window.set_focusable(true).map_err(error);
        shown?;
        restored?;
    }
    Ok(())
}

#[tauri::command]
async fn prepare_codex_hook(
    runtime: State<'_, Arc<Runtime>>,
    session_id: String,
) -> CommandResult<crate::codex_dispatcher::Setup> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let info = runtime
            .dashboard()
            .map_err(error)?
            .sessions
            .into_iter()
            .find(|s| {
                s.id == session_id && s.provider_id == "codex" && (s.read_only || s.mode == "hook")
            })
            .ok_or("Select an existing Codex session")?;
        let executable = std::env::current_exe().map_err(error)?;
        let setup = crate::codex_dispatcher::prepare(&runtime, &info.profile_id, &executable)
            .map_err(error)?;
        crate::codex_dispatcher::reconnect_session(&runtime, info).map_err(error)?;
        Ok(setup)
    })
    .await
    .map_err(error)?
}
#[tauri::command]
async fn get_usage(
    runtime: State<'_, Arc<Runtime>>,
    intervals: Vec<crate::usage::Interval>,
    profile_id: Option<String>,
) -> CommandResult<crate::usage::UsageReport> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = runtime.usage_lock.lock().unwrap();
        let profiles = match profile_id {
            Some(id) => vec![runtime.profile(&id).map_err(error)?.0],
            None => runtime.store.profiles().map_err(error)?,
        };
        let settings = runtime.settings.read().unwrap().clone();
        crate::usage::collect(
            &runtime.store,
            &profiles,
            &runtime.config.providers,
            &settings,
            &dirs::home_dir().unwrap_or_default(),
            &std::env::vars().collect(),
            &intervals,
        )
        .map_err(error)
    })
    .await
    .map_err(error)?
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
    let mode_changed = settings.window_mode != runtime.settings.read().unwrap().window_mode;
    runtime.save_settings(settings).map_err(error)?;
    let view = app.state::<NativeState>().view.lock().unwrap().clone();
    if let Some((view, _)) = view {
        layout(&app, &runtime, if mode_changed { "widget" } else { &view })?;
    }
    if runtime.settings.read().unwrap().window_mode == "floating" {
        let target = app.state::<NativeState>().panel.lock().unwrap().clone();
        if let Some(target) = target {
            position_dock_panel(&app, &target)?;
        }
    } else {
        *app.state::<NativeState>().panel.lock().unwrap() = None;
        if let Some(panel) = app.get_webview_window("dock-panel") {
            let _ = panel.hide();
        }
    }
    Ok(())
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
    if runtime
        .sessions
        .list()
        .iter()
        .any(|s| s.id == session_id && s.mode == "hook")
    {
        crate::codex_dispatcher::disconnect_session(&runtime, &session_id).map_err(error)
    } else {
        runtime.sessions.stop(&session_id).map_err(error)
    }
}
#[tauri::command]
fn disable_codex_hooks(runtime: State<'_, Arc<Runtime>>, profile_id: String) -> CommandResult<()> {
    crate::codex_dispatcher::disable(&runtime, &profile_id).map_err(error)
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
            let side = native
                .placed_dock_side
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| runtime.settings.read().unwrap().dock_side.clone());
            format!("dock:{side}")
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
    let window = app
        .get_webview_window(if view == "workspace" {
            "workspace"
        } else {
            "main"
        })
        .ok_or("Window unavailable")?;
    if view != "workspace" {
        save_geometry(app, runtime);
    }
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
        let size = s.dock_hover_size;
        let rail_width = size + s.dock_padding;
        let height = 2.0 * s.dock_padding
            + count as f64 * size
            + count.saturating_sub(1) as f64 * s.dock_gap
            + s.dock_chrome_height;
        let width = rail_width;
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
                    .window_bounds(&format!("dock:{}", s.dock_side))
                    .ok()
                    .flatten()
                    .or_else(|| runtime.store.window_bounds("dock").ok().flatten())
                    .map(|b| b.y),
            );
            let _ = window.set_min_size(Some(LogicalSize::new(24.0, 80.0)));
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
        *app.state::<NativeState>().placed_dock_side.lock().unwrap() = Some(s.dock_side.clone());
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
    let desired = if view == "workspace" {
        saved.map(|b| (b.width, b.height)).unwrap_or(desired)
    } else {
        desired
    };
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
    let _ = window.set_always_on_top(if view == "workspace" {
        false
    } else {
        s.always_on_top
    });
    if view != "workspace" {
        *app.state::<NativeState>().view.lock().unwrap() = Some((view.into(), island));
    }
    window.show().map_err(error)?;
    Ok(())
}

#[tauri::command]
fn set_window_view(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    view: String,
    window: tauri::WebviewWindow,
) -> CommandResult<()> {
    if window.label() != "main" || view == "workspace" {
        return Err("Use the independent workspace window".into());
    }
    layout(&app, &runtime, &view)
}
#[tauri::command]
fn hide_window(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    window: tauri::WebviewWindow,
) -> CommandResult<()> {
    if window.label() == "workspace" {
        return window.hide().map_err(error);
    }
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
fn open_workspace(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    page: String,
    profile_id: Option<String>,
    session_id: Option<String>,
) -> CommandResult<()> {
    layout(&app, &runtime, "workspace")?;
    let window = app
        .get_webview_window("workspace")
        .ok_or("Workspace unavailable")?;
    window
        .emit(
            "orbit:navigate",
            serde_json::json!({"page":page,"profileId":profile_id,"sessionId":session_id}),
        )
        .map_err(error)?;
    window.set_focus().map_err(error)
}
#[tauri::command]
fn drag_dock(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    window: tauri::WebviewWindow,
) -> CommandResult<()> {
    if window.label() != "main" || runtime.settings.read().unwrap().dock_locked {
        return Err("Dock position is locked".into());
    }
    let position = window.inner_position().map_err(error)?;
    let scale = window.scale_factor().map_err(error)?;
    *app.state::<NativeState>().drag_origin.lock().unwrap() = Some(Bounds {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
        width: 0.0,
        height: 0.0,
    });
    app.state::<NativeState>()
        .dragging
        .store(true, Ordering::Relaxed);
    if let Some(panel) = app.get_webview_window("dock-panel") {
        let _ = panel.hide();
    }
    Ok(())
}
#[tauri::command]
fn move_dock(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    dx: f64,
    dy: f64,
) -> CommandResult<()> {
    if window.label() != "main" || !dx.is_finite() || !dy.is_finite() {
        return Err("Invalid dock movement".into());
    }
    let origin = *app.state::<NativeState>().drag_origin.lock().unwrap();
    if let Some(origin) = origin {
        window
            .set_position(LogicalPosition::new(origin.x + dx, origin.y + dy))
            .map_err(error)?;
    }
    Ok(())
}
#[tauri::command]
fn finish_dock_drag(app: tauri::AppHandle, window: tauri::WebviewWindow) -> CommandResult<()> {
    if window.label() != "main" {
        return Err("Dock window required".into());
    }
    *app.state::<NativeState>().drag_origin.lock().unwrap() = None;
    if app
        .state::<NativeState>()
        .dragging
        .swap(false, Ordering::Relaxed)
    {
        snap_dock(&app);
    }
    Ok(())
}
#[tauri::command]
fn reset_dock_position(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
) -> CommandResult<()> {
    let side = runtime.settings.read().unwrap().dock_side.clone();
    let Some(window) = app.get_webview_window("main") else {
        return Err("Dock unavailable".into());
    };
    if let Some(m) = window.current_monitor().map_err(error)? {
        let scale = m.scale_factor();
        let size = window.inner_size().map_err(error)?;
        let display = Bounds {
            x: m.position().x as f64 / scale,
            y: m.position().y as f64 / scale,
            width: m.size().width as f64 / scale,
            height: m.size().height as f64 / scale,
        };
        let inset = runtime.settings.read().unwrap().dock_edge_inset;
        let bounds = crate::window_state::dock_bounds(
            display,
            size.width as f64 / scale,
            size.height as f64 / scale,
            &side,
            inset,
        );
        window
            .set_position(LogicalPosition::new(bounds.x, bounds.y))
            .map_err(error)?;
        save_geometry(&app, &runtime);
    }
    Ok(())
}
fn position_dock_panel(app: &tauri::AppHandle, target: &PanelTarget) -> CommandResult<()> {
    let runtime = app.state::<Arc<Runtime>>();
    let s = runtime.settings.read().unwrap().clone();
    let dock = app.get_webview_window("main").ok_or("Dock unavailable")?;
    let panel = app
        .get_webview_window("dock-panel")
        .ok_or("Detail window unavailable")?;
    let Some(m) = dock.current_monitor().map_err(error)? else {
        return Ok(());
    };
    let scale = dock.scale_factor().map_err(error)?;
    let ms = m.scale_factor();
    let p = dock.inner_position().map_err(error)?;
    let z = dock.inner_size().map_err(error)?;
    let display = Bounds {
        x: m.position().x as f64 / ms,
        y: m.position().y as f64 / ms,
        width: m.size().width as f64 / ms,
        height: m.size().height as f64 / ms,
    };
    let bounds = crate::window_state::panel_bounds(
        display,
        Bounds {
            x: p.x as f64 / scale,
            y: p.y as f64 / scale,
            width: z.width as f64 / scale,
            height: z.height as f64 / scale,
        },
        s.dock_panel_width,
        target.height.min(s.dock_popup_height),
        &s.dock_side,
        s.dock_gap,
        target.anchor,
    );
    panel
        .set_size(LogicalSize::new(bounds.width, bounds.height))
        .map_err(error)?;
    panel
        .set_position(LogicalPosition::new(bounds.x, bounds.y))
        .map_err(error)?;
    panel.set_always_on_top(s.always_on_top).map_err(error)?;
    Ok(())
}
#[tauri::command]
fn set_dock_panel(
    app: tauri::AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    profile_id: Option<String>,
    anchor: f64,
    pinned: bool,
    focus: Option<bool>,
    window: tauri::WebviewWindow,
) -> CommandResult<()> {
    if window.label() != "main" || !anchor.is_finite() {
        return Err("Invalid detail target".into());
    }
    show_dock_panel(&app, &runtime, profile_id, anchor, pinned)?;
    if focus.unwrap_or(false) {
        if let Some(panel) = app.get_webview_window("dock-panel") {
            panel.set_focus().map_err(error)?;
        }
    }
    Ok(())
}

fn show_dock_panel(
    app: &tauri::AppHandle,
    runtime: &Runtime,
    profile_id: Option<String>,
    anchor: f64,
    pinned: bool,
) -> CommandResult<()> {
    let panel = app
        .get_webview_window("dock-panel")
        .ok_or("Detail window unavailable")?;
    let profile_id = if runtime.settings.read().unwrap().window_mode == "floating" {
        profile_id
    } else {
        None
    };
    if let Some(profile_id) = profile_id {
        runtime.profile(&profile_id).map_err(error)?;
        let previous = app.state::<NativeState>().panel.lock().unwrap().clone();
        let target = PanelTarget {
            height: previous
                .filter(|p| p.profile_id == profile_id)
                .map(|p| p.height)
                .unwrap_or(runtime.settings.read().unwrap().dock_popup_height),
            profile_id,
            anchor,
            pinned,
        };
        *app.state::<NativeState>().panel.lock().unwrap() = Some(target.clone());
        position_dock_panel(&app, &target)?;
        panel.emit("orbit:dock-panel", &target).map_err(error)?;
        if !panel.is_visible().map_err(error)? {
            // Showing a hover window must not take focus from the user's editor.
            panel.set_focusable(false).map_err(error)?;
            let shown = panel.show().map_err(error);
            let restored = panel.set_focusable(true).map_err(error);
            shown?;
            restored?;
        }
        // An explicit click permits keyboard input in approval question fields.
        // Automatic and hover opens remain non-activating.
        if pinned {
            panel.set_focus().map_err(error)?;
        }
    } else {
        *app.state::<NativeState>().panel.lock().unwrap() = None;
        panel.hide().map_err(error)?;
    }
    Ok(())
}
fn pending_announcement<'a>(
    pending: &'a [Interaction],
    seen: &std::collections::HashSet<String>,
) -> Option<&'a Interaction> {
    pending
        .iter()
        .filter(|r| !seen.contains(&r.id))
        .min_by_key(|r| (r.created_at, &r.id))
}

// Approval arrival is a native window operation, independent of workspace visibility
// and of the rail webview's hover lifecycle.
fn announce_approvals(app: &tauri::AppHandle) -> CommandResult<()> {
    let runtime = app.state::<Arc<Runtime>>();
    let pending: Vec<_> = runtime
        .broker
        .list()
        .into_iter()
        .filter(|r| r.expires_at > chrono::Utc::now().timestamp())
        .collect();
    let state = app.state::<NativeState>();
    let next = {
        let mut seen = state.announced_approvals.lock().unwrap();
        seen.retain(|id| pending.iter().any(|r| &r.id == id));
        pending_announcement(&pending, &seen).cloned()
    };
    let Some(request) = next else {
        return Ok(());
    };
    if runtime.settings.read().unwrap().window_mode != "floating" {
        return Ok(());
    }
    let sessions = runtime.sessions.list();
    let Some(session) = sessions
        .iter()
        .find(|s| s.id == request.session_id && !s.read_only)
    else {
        return Ok(());
    };
    let settings = runtime.settings.read().unwrap().clone();
    let profiles = runtime.store.profiles().map_err(error)?;
    let mut visible: Vec<_> = profiles
        .iter()
        .filter(|p| {
            (p.enabled && settings.visible_providers.contains(&p.provider_id))
                || sessions
                    .iter()
                    .any(|s| s.profile_id == p.id && pending.iter().any(|r| r.session_id == s.id))
        })
        .collect();
    visible.sort_by_key(|p| {
        settings
            .visible_providers
            .iter()
            .position(|id| id == &p.provider_id)
            .unwrap_or(usize::MAX)
    });
    let Some(index) = visible.iter().position(|p| p.id == session.profile_id) else {
        return Ok(());
    };
    let anchor = settings.dock_padding
        + settings.dock_chrome_height * 0.25
        + index as f64 * (settings.dock_hover_size + settings.dock_gap)
        + settings.dock_hover_size / 2.0;
    show_dock_panel(
        app,
        &runtime,
        Some(session.profile_id.clone()),
        anchor,
        false,
    )?;
    state
        .announced_approvals
        .lock()
        .unwrap()
        .extend(pending.iter().map(|r| r.id.clone()));
    if let Some(main) = app.get_webview_window("main") {
        main.emit(
            "orbit:panel-presence",
            serde_json::json!({"inside":false,"profileId":session.profile_id}),
        )
        .map_err(error)?;
    }
    Ok(())
}

#[tauri::command]
fn get_dock_panel(app: tauri::AppHandle) -> Option<PanelTarget> {
    app.state::<NativeState>().panel.lock().unwrap().clone()
}
#[tauri::command]
fn resize_dock_panel(
    app: tauri::AppHandle,
    height: f64,
    profile_id: String,
    window: tauri::WebviewWindow,
) -> CommandResult<()> {
    if window.label() != "dock-panel" || !height.is_finite() || height <= 0.0 {
        return Err("Invalid detail size".into());
    }
    let target = {
        let native = app.state::<NativeState>();
        let mut panel = native.panel.lock().unwrap();
        if let Some(target) = panel
            .as_mut()
            .filter(|target| target.profile_id == profile_id)
        {
            target.height = height;
        }
        panel.clone()
    };
    if let Some(target) = target {
        position_dock_panel(&app, &target)?;
    }
    Ok(())
}
#[tauri::command]
fn dock_panel_pointer(
    app: tauri::AppHandle,
    inside: bool,
    dismiss: Option<bool>,
    toggle_pin: Option<bool>,
) -> CommandResult<()> {
    if dismiss.unwrap_or(false) {
        *app.state::<NativeState>().panel.lock().unwrap() = None;
        if let Some(panel) = app.get_webview_window("dock-panel") {
            let _ = panel.hide();
        }
    }
    if let Some(main) = app.get_webview_window("main") {
        main.emit("orbit:panel-presence",serde_json::json!({"inside":inside,"dismiss":dismiss.unwrap_or(false),"togglePin":toggle_pin.unwrap_or(false)})).map_err(error)?;
    }
    Ok(())
}
fn show_workspace(app: &tauri::AppHandle) {
    let runtime = app.state::<Arc<Runtime>>();
    let _ = layout(app, &runtime, "workspace");
    if let Some(window) = app.get_webview_window("workspace") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn snap_dock(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let runtime = app.state::<Arc<Runtime>>();
    let (Ok(position), Ok(size), Ok(scale), Ok(Some(monitor))) = (
        window.inner_position(),
        window.inner_size(),
        window.scale_factor(),
        window.current_monitor(),
    ) else {
        return;
    };
    let monitor_scale = monitor.scale_factor();
    let display = Bounds {
        x: monitor.position().x as f64 / monitor_scale,
        y: monitor.position().y as f64 / monitor_scale,
        width: monitor.size().width as f64 / monitor_scale,
        height: monitor.size().height as f64 / monitor_scale,
    };
    let mut settings = runtime.settings.read().unwrap().clone();
    let view = app
        .state::<NativeState>()
        .view
        .lock()
        .unwrap()
        .clone()
        .map(|(v, _)| v)
        .unwrap_or_else(|| "dock".into());
    let rail = settings.dock_hover_size + settings.dock_padding;
    let width = size.width as f64 / scale;
    let rail_x = position.x as f64 / scale
        + if settings.dock_side == "right" {
            width - rail
        } else {
            0.0
        };
    settings.dock_side = crate::window_state::nearest_dock_side(display, rail_x, rail).into();
    save_geometry(app, &runtime);
    let _ = runtime.store.save_window_bounds(
        &format!("dock:{}", settings.dock_side),
        Bounds {
            x: position.x as f64 / scale,
            y: position.y as f64 / scale,
            width: size.width as f64 / scale,
            height: size.height as f64 / scale,
        },
    );
    if runtime.save_settings(settings).is_ok() {
        let _ = layout(app, &runtime, &view);
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle, runtime: State<'_, Arc<Runtime>>) {
    save_geometry(&app, &runtime);
    runtime.sessions.stop_all();
    app.exit(0);
}

fn window_cursor(window: &tauri::WebviewWindow, cursor: (f64, f64)) -> Option<(f64, f64)> {
    if !window.is_visible().ok()? {
        return None;
    }
    let position = window.inner_position().ok()?;
    let size = window.inner_size().ok()?;
    crate::window_state::cursor_in_window(
        cursor,
        Bounds {
            x: position.x as f64,
            y: position.y as f64,
            width: size.width as f64,
            height: size.height as f64,
        },
        window.scale_factor().ok()?,
    )
}

fn sample_dock_pointer(app: &tauri::AppHandle) {
    let state = app.state::<NativeState>();
    if state.dragging.load(Ordering::Relaxed) {
        return;
    }
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    if !main.is_visible().unwrap_or(false) {
        return;
    }
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    let cursor = (cursor.x, cursor.y);
    let local = window_cursor(&main, cursor);
    let panel_inside = app
        .get_webview_window("dock-panel")
        .and_then(|panel| window_cursor(&panel, cursor))
        .is_some();
    let _ = main.emit(
        "orbit:dock-pointer",
        serde_json::json!({
            "x":local.map(|p| p.0), "y":local.map(|p| p.1), "panelInside":panel_inside
        }),
    );
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
            get_task_completions,
            dismiss_task_completion,
            prepare_codex_hook,
            disable_codex_hooks,
            get_usage,
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
            open_workspace,
            drag_dock,
            move_dock,
            finish_dock_drag,
            reset_dock_position,
            set_dock_panel,
            get_dock_panel,
            resize_dock_panel,
            dock_panel_pointer,
            hide_window,
            quit_app
        ])
        .on_window_event(|window, event| {
            let app = window.app_handle();
            if window.label() == "task-completion" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let id = app
                        .state::<NativeState>()
                        .shown_completion
                        .lock()
                        .unwrap()
                        .clone();
                    if let Some(id) = id {
                        app.state::<Arc<Runtime>>().sessions.dismiss_completion(&id);
                    }
                    let _ = sync_completion_popup(app);
                }
                return;
            }
            if window.label() == "workspace" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                if matches!(
                    event,
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                ) {
                    if let (Ok(p), Ok(s), Ok(scale)) = (
                        window.inner_position(),
                        window.inner_size(),
                        window.scale_factor(),
                    ) {
                        let _ = app.state::<Arc<Runtime>>().store.save_window_bounds(
                            "workspace",
                            Bounds {
                                x: p.x as f64 / scale,
                                y: p.y as f64 / scale,
                                width: s.width as f64 / scale,
                                height: s.height as f64 / scale,
                            },
                        );
                    }
                }
                return;
            }
            if window.label() == "dock-panel" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = dock_panel_pointer(app.clone(), false, Some(true), None);
                }
            }
        })
        .setup(move |app| {
            startup_trace("native setup callback");
            let handle = app.handle().clone();
            let events_handle = handle.clone();
            let mut events = rt.events.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(event) => {
                            if event.kind == "dashboard" {
                                if let Err(error) = sync_completion_popup(&events_handle) {
                                    eprintln!("Completion popup unavailable: {error}");
                                }
                                if let Err(error) = announce_approvals(&events_handle) {
                                    eprintln!("Approval popup unavailable: {error}");
                                }
                            }
                            let _ = events_handle.emit("orbit:event", event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = sync_completion_popup(&events_handle);
                            let _ = announce_approvals(&events_handle);
                            let _ = events_handle.emit(
                                "orbit:event",
                                AppEvent {
                                    kind: "dashboard".into(),
                                    session_id: None,
                                    data: serde_json::Value::Null,
                                },
                            );
                        }
                        Err(_) => break,
                    }
                }
            });
            let pointer_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let (mode, interval) = {
                        let runtime = pointer_handle.state::<Arc<Runtime>>();
                        let settings = runtime.settings.read().unwrap();
                        (settings.window_mode.clone(), settings.dock_pointer_poll_ms)
                    };
                    if mode == "floating" {
                        sample_dock_pointer(&pointer_handle);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
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
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_workspace(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_workspace(app),
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
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_workspace(app);
        }
        if let tauri::RunEvent::ExitRequested { .. } = event {
            save_geometry(app, &app.state::<Arc<Runtime>>());
            app.state::<Arc<Runtime>>().sessions.stop_all();
        }
    });
}

#[cfg(test)]
mod approval_popup_tests {
    use super::*;
    fn request(id: &str, created_at: i64) -> Interaction {
        Interaction {
            profile_label: String::new(),
            cwd: String::new(),
            id: id.into(),
            session_id: "managed".into(),
            provider_id: "codex".into(),
            title: String::new(),
            detail: String::new(),
            kind: "approval".into(),
            choices: vec![],
            questions: vec![],
            created_at,
            expires_at: i64::MAX,
        }
    }
    #[test]
    fn new_approval_opens_once_and_later_requests_reopen() {
        let mut seen = std::collections::HashSet::new();
        let first = vec![request("one", 1)];
        assert_eq!(pending_announcement(&first, &seen).unwrap().id, "one");
        seen.insert("one".into());
        assert!(pending_announcement(&first, &seen).is_none());
        let next = vec![request("two", 2), request("one", 1)];
        assert_eq!(pending_announcement(&next, &seen).unwrap().id, "two");
        assert!(pending_announcement(&[], &seen).is_none());
    }
    #[test]
    fn concurrent_requests_open_oldest_unannounced_first() {
        let pending = vec![request("later", 2), request("first", 1)];
        assert_eq!(
            pending_announcement(&pending, &Default::default())
                .unwrap()
                .id,
            "first"
        );
    }
}
