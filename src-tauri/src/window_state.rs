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
