//! Keep Tauri geometry/IPC anchors, but display real accessory panels on macOS.
use std::ops::Deref;
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager};

#[derive(Default)]
pub struct OverlayWindows {
    #[cfg(target_os = "macos")]
    visible: std::sync::Mutex<std::collections::HashMap<String, bool>>,
}

#[derive(Clone)]
pub struct OrbitWindow(tauri::WebviewWindow);

pub trait OrbitManager: Manager<tauri::Wry> {
    fn get_orbit_window(&self, label: &str) -> Option<OrbitWindow> {
        self.get_webview_window(label).map(OrbitWindow::new)
    }
}
impl<T: Manager<tauri::Wry>> OrbitManager for T {}

impl Deref for OrbitWindow {
    type Target = tauri::WebviewWindow;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OrbitWindow {
    pub fn new(window: tauri::WebviewWindow) -> Self {
        Self(window)
    }
    fn helper(&self) -> bool {
        cfg!(target_os = "macos") && crate::overlay_windows::is_overlay(self.label())
    }
    fn send(&self, action: &str, mut values: serde_json::Value) -> Result<(), String> {
        if !self.helper() {
            return Ok(());
        }
        values["type"] = "window".into();
        values["label"] = self.label().into();
        values["action"] = action.into();
        #[cfg(target_os = "macos")]
        {
            let transport = self
                .app_handle()
                .try_state::<crate::overlay_transport::OverlayTransport>()
                .ok_or("Native overlay helper unavailable; use the workspace")?;
            transport.send(values)?;
        }
        Ok(())
    }
    pub fn show(&self) -> Result<(), String> {
        if !self.helper() {
            return self.0.show().map_err(|e| e.to_string());
        }
        self.send("show", serde_json::json!({}))?;
        #[cfg(target_os = "macos")]
        self.app_handle()
            .state::<OverlayWindows>()
            .visible
            .lock()
            .unwrap()
            .insert(self.label().into(), true);
        Ok(())
    }
    pub fn hide(&self) -> Result<(), String> {
        if !self.helper() {
            return self.0.hide().map_err(|e| e.to_string());
        }
        self.send("hide", serde_json::json!({}))?;
        #[cfg(target_os = "macos")]
        self.app_handle()
            .state::<OverlayWindows>()
            .visible
            .lock()
            .unwrap()
            .insert(self.label().into(), false);
        Ok(())
    }
    pub fn is_visible(&self) -> Result<bool, String> {
        if !self.helper() {
            return self.0.is_visible().map_err(|e| e.to_string());
        }
        #[cfg(target_os = "macos")]
        return Ok(*self
            .app_handle()
            .state::<OverlayWindows>()
            .visible
            .lock()
            .unwrap()
            .get(self.label())
            .unwrap_or(&false));
        #[cfg(not(target_os = "macos"))]
        unreachable!()
    }
    pub fn set_focus(&self) -> Result<(), String> {
        if !self.helper() {
            return self.0.set_focus().map_err(|e| e.to_string());
        }
        self.send("focus", serde_json::json!({}))
    }
    pub fn set_position(&self, position: LogicalPosition<f64>) -> Result<(), String> {
        self.0.set_position(position).map_err(|e| e.to_string())?;
        self.send(
            "position",
            serde_json::json!({"x":position.x,"y":position.y}),
        )
    }
    pub fn set_size(&self, size: LogicalSize<f64>) -> Result<(), String> {
        self.0.set_size(size).map_err(|e| e.to_string())?;
        self.send(
            "size",
            serde_json::json!({"width":size.width,"height":size.height}),
        )
    }
    pub fn set_min_size(&self, size: Option<LogicalSize<f64>>) -> Result<(), String> {
        self.0.set_min_size(size).map_err(|e| e.to_string())?;
        let size = size.unwrap_or(LogicalSize::new(1.0, 1.0));
        self.send(
            "minSize",
            serde_json::json!({"width":size.width,"height":size.height}),
        )
    }
    pub fn set_always_on_top(&self, value: bool) -> Result<(), String> {
        self.0.set_always_on_top(value).map_err(|e| e.to_string())?;
        self.send("top", serde_json::json!({"value":value}))
    }
    pub fn set_shadow(&self, value: bool) -> Result<(), String> {
        self.0.set_shadow(value).map_err(|e| e.to_string())?;
        self.send("shadow", serde_json::json!({"value":value}))
    }
    pub fn set_resizable(&self, value: bool) -> Result<(), String> {
        self.0.set_resizable(value).map_err(|e| e.to_string())?;
        self.send("resizable", serde_json::json!({"value":value}))
    }
    pub fn set_focusable(&self, value: bool) -> Result<(), String> {
        self.0.set_focusable(value).map_err(|e| e.to_string())?;
        self.send("focusable", serde_json::json!({"value":value}))
    }
    pub fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), String> {
        self.0
            .emit(event, payload.clone())
            .map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        if self.helper() {
            if let Some(transport) = self
                .app_handle()
                .try_state::<crate::overlay_transport::OverlayTransport>()
            {
                transport.send(serde_json::json!({"type":"event","label":self.label(),"event":event,"payload":payload}))?;
            }
        }
        Ok(())
    }
}
