//! Platform-independent validation for the private overlay pipe protocol.
use std::io::{self, BufRead};
pub const MAX_FRAME: usize = 8 * 1024 * 1024;
pub const MAX_PENDING: usize = 128;
pub const MAX_JS_ID: u64 = 9_007_199_254_740_991;
pub fn valid_label(label: &str) -> bool {
    matches!(label, "main" | "dock-panel" | "task-completion")
}
pub fn valid_asset_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b))
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
pub fn allowed_command(label: &str, command: &str) -> bool {
    if !valid_label(label) {
        return false;
    }
    if label == "task-completion" {
        return matches!(
            command,
            "get_task_completions" | "dismiss_task_completion" | "open_workspace" | "get_dashboard"
        );
    }
    match command {
        "set_window_view" | "drag_dock" | "move_dock" | "finish_dock_drag" | "set_dock_panel" => {
            label == "main"
        }
        "resize_dock_panel" => label == "dock-panel",
        "get_dashboard"
        | "get_task_completions"
        | "dismiss_task_completion"
        | "get_usage"
        | "refresh_profile"
        | "save_profile"
        | "save_settings"
        | "start_session"
        | "write_session"
        | "prompt_session"
        | "resize_session"
        | "session_replay"
        | "stop_session"
        | "respond"
        | "open_workspace"
        | "reset_dock_position"
        | "get_dock_panel"
        | "dock_panel_pointer"
        | "hide_window"
        | "quit_app"
        | "prepare_codex_hook"
        | "disable_codex_hooks"
        | "overlay_open_url"
        | "overlay_choose_directory" => true,
        _ => false,
    }
}
/// Read only up to the frame cap; never let read_line allocate an unbounded frame.
pub fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "truncated overlay frame",
                ))
            };
        }
        let newline = buf.iter().position(|b| *b == b'\n');
        let len = newline.unwrap_or(buf.len());
        if frame.len() + len > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "overlay frame exceeds limit",
            ));
        }
        frame.extend_from_slice(&buf[..len]);
        reader.consume(len + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(frame));
        }
    }
}
pub fn valid_geometry(x: f64, y: f64, width: f64, height: f64) -> bool {
    [x, y, width, height].iter().all(|v| v.is_finite())
        && x.abs() <= 100_000.0
        && y.abs() <= 100_000.0
        && width > 0.0
        && width <= 32_768.0
        && height > 0.0
        && height <= 32_768.0
}
/// A native directory dialog is explicitly user-controlled and may remain open.
pub fn invocation_has_deadline(command: &str) -> bool {
    command != "overlay_choose_directory"
}
pub fn valid_content_error(label: &str, error: &str) -> bool {
    valid_label(label) && error.len() <= 4096
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_failure_requires_known_label_and_bounded_diagnostic() {
        assert!(valid_content_error("main", "navigation failed"));
        assert!(valid_content_error("dock-panel", &"x".repeat(4096)));
        assert!(!valid_content_error("workspace", "error"));
        assert!(!valid_content_error("main", &"x".repeat(4097)));
    }
    #[test]
    fn only_directory_picker_can_wait_for_user_without_deadline() {
        assert!(!invocation_has_deadline("overlay_choose_directory"));
        for command in [
            "get_dashboard",
            "start_session",
            "overlay_open_url",
            "unknown",
        ] {
            assert!(invocation_has_deadline(command));
        }
    }
    #[test]
    fn geometry_is_finite_bounded_and_positive() {
        assert!(valid_geometry(-4000.0, 200.0, 420.0, 480.0));
        for (x, y, w, h) in [
            (f64::NAN, 0.0, 1.0, 1.0),
            (0.0, f64::INFINITY, 1.0, 1.0),
            (0.0, 0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, -1.0),
            (0.0, 0.0, 1e9, 1.0),
            (1e9, 0.0, 1.0, 1.0),
        ] {
            assert!(!valid_geometry(x, y, w, h));
        }
    }
    #[test]
    fn asset_paths_reject_traversal() {
        for p in [
            "",
            "/etc/passwd",
            "../secret",
            "a/../b",
            "a\\b",
            "a?b",
            "a#b",
            "%2e%2e/secret",
            "a//b",
            "./a",
            "a\0b",
            "C:/secret",
        ] {
            assert!(!valid_asset_path(p), "{p:?}");
        }
        assert!(valid_asset_path("index.html"));
        assert!(valid_asset_path("assets/main-abc.js"));
    }
    #[test]
    fn role_permissions_are_narrow() {
        assert!(allowed_command("main", "start_session"));
        assert!(allowed_command("dock-panel", "get_usage"));
        assert!(allowed_command(
            "task-completion",
            "dismiss_task_completion"
        ));
        for (l, c) in [
            ("unknown", "get_dashboard"),
            ("main", "plugin:shell|execute"),
            ("dock-panel", "drag_dock"),
            ("task-completion", "start_session"),
            ("main", "resize_dock_panel"),
        ] {
            assert!(!allowed_command(l, c), "{l}:{c}");
        }
    }
    #[test]
    fn frames_require_newline_and_are_bounded() {
        assert_eq!(
            read_frame(&mut &b"hello\nrest\n"[..]).unwrap(),
            Some(b"hello".to_vec())
        );
        assert!(read_frame(&mut &b"unterminated"[..]).is_err());
        assert!(read_frame(&mut &vec![b'x'; MAX_FRAME + 1][..]).is_err());
        assert_eq!(read_frame(&mut &b""[..]).unwrap(), None);
    }
}
