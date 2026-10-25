use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
pub fn restore(saved: Option<Bounds>, desired: (f64, f64), displays: &[Bounds]) -> Bounds {
    let Some(first) = displays.first() else {
        return Bounds {
            x: 0.0,
            y: 0.0,
            width: desired.0,
            height: desired.1,
        };
    };
    let display = saved
        .and_then(|s| {
            displays.iter().find(|d| {
                s.x + s.width / 2.0 >= d.x
                    && s.x + s.width / 2.0 < d.x + d.width
                    && s.y + s.height / 2.0 >= d.y
                    && s.y + s.height / 2.0 < d.y + d.height
            })
        })
        .unwrap_or(first);
    let width = desired.0.min(display.width);
    let height = desired.1.min(display.height);
    let visible = saved.filter(|s| {
        s.x < display.x + display.width
            && s.y < display.y + display.height
            && s.x + s.width > display.x
            && s.y + s.height > display.y
    });
    let (x, y) = visible
        .map(|s| {
            (
                s.x.clamp(display.x, display.x + display.width - width),
                s.y.clamp(display.y, display.y + display.height - height),
            )
        })
        .unwrap_or((
            display.x + (display.width - width) / 2.0,
            display.y + (display.height - height) / 2.0,
        ));
    Bounds {
        x,
        y,
        width,
        height,
    }
}
pub fn dock_bounds(display: Bounds, width: f64, height: f64, side: &str, inset: f64) -> Bounds {
    let inset = inset
        .max(0.0)
        .min(display.width / 4.0)
        .min(display.height / 4.0);
    let width = width.min(display.width - 2.0 * inset);
    let height = height.min(display.height - 2.0 * inset);
    Bounds {
        x: if side == "left" {
            display.x + inset
        } else {
            display.x + display.width - inset - width
        },
        y: display.y + (display.height - height) / 2.0,
        width,
        height,
    }
}
pub fn dock_bounds_at(
    display: Bounds,
    width: f64,
    height: f64,
    side: &str,
    inset: f64,
    vertical: Option<f64>,
) -> Bounds {
    let mut bounds = dock_bounds(display, width, height, side, inset);
    let inset = inset
        .max(0.0)
        .min(display.width / 4.0)
        .min(display.height / 4.0);
    if let Some(y) = vertical.filter(|y| y.is_finite()) {
        bounds.y = y.clamp(
            display.y + inset,
            display.y + display.height - inset - bounds.height,
        );
    }
    bounds
}

pub fn nearest_dock_side(display: Bounds, x: f64, rail_width: f64) -> &'static str {
    if x + rail_width / 2.0 < display.x + display.width / 2.0 {
        "left"
    } else {
        "right"
    }
}

pub fn panel_bounds(
    display: Bounds,
    dock: Bounds,
    width: f64,
    height: f64,
    side: &str,
    gap: f64,
    anchor: f64,
) -> Bounds {
    let width = width.min(display.width).max(1.0);
    let height = height.min(display.height).max(1.0);
    let x = if side == "left" {
        dock.x + dock.width + gap
    } else {
        dock.x - gap - width
    };
    Bounds {
        x: x.clamp(display.x, display.x + display.width - width),
        y: (dock.y + anchor - height / 2.0).clamp(display.y, display.y + display.height - height),
        width,
        height,
    }
}

/// Convert physical desktop coordinates into a window's logical client coordinates.
pub fn cursor_in_window(cursor: (f64, f64), window: Bounds, scale: f64) -> Option<(f64, f64)> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let x = cursor.0 - window.x;
    let y = cursor.1 - window.y;
    if x >= 0.0 && y >= 0.0 && x < window.width && y < window.height {
        Some((x / scale, y / scale))
    } else {
        None
    }
}
