use agent_orbit::window_state::{restore, Bounds};
fn display() -> Bounds {
    Bounds {
        x: 0.0,
        y: 0.0,
        width: 1440.0,
        height: 900.0,
    }
}
#[test]
fn disconnected_monitor_position_is_recovered() {
    let result = restore(
        Some(Bounds {
            x: 3000.0,
            y: 500.0,
            width: 430.0,
            height: 640.0,
        }),
        (430.0, 640.0),
        &[display()],
    );
    assert_eq!(result.x, 505.0);
    assert_eq!(result.y, 130.0);
}
#[test]
fn visible_saved_position_is_preserved() {
    let result = restore(
        Some(Bounds {
            x: 40.0,
            y: 60.0,
            width: 430.0,
            height: 640.0,
        }),
        (430.0, 640.0),
        &[display()],
    );
    assert_eq!(result.x, 40.0);
    assert_eq!(result.y, 60.0);
}
#[test]
fn oversized_windows_fit_the_display() {
    let result = restore(None, (2000.0, 1500.0), &[display()]);
    assert_eq!(result.width, 1440.0);
    assert_eq!(result.height, 900.0);
}
