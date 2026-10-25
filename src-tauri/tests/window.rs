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
#[test]
fn expanding_right_dock_preserves_its_screen_edge() {
    let a = agent_orbit::window_state::dock_bounds(display(), 80.0, 580.0, "right", 12.0);
    let b = agent_orbit::window_state::dock_bounds(display(), 452.0, 580.0, "right", 12.0);
    assert_eq!(a.x + a.width, b.x + b.width);
    assert_eq!(a.x + a.width, 1428.0);
    assert_eq!(a.y, b.y);
}
#[test]
fn left_dock_expands_inward_and_fits_small_screens() {
    let a = agent_orbit::window_state::dock_bounds(display(), 80.0, 580.0, "left", 12.0);
    let b = agent_orbit::window_state::dock_bounds(display(), 452.0, 1200.0, "left", 12.0);
    assert_eq!(a.x, 12.0);
    assert_eq!(a.x, b.x);
    assert!(b.y >= 0.0 && b.y + b.height <= 900.0);
}
#[test]
fn dragged_dock_height_survives_hover_expansion() {
    let a = agent_orbit::window_state::dock_bounds_at(
        display(),
        80.0,
        580.0,
        "right",
        12.0,
        Some(40.0),
    );
    let b = agent_orbit::window_state::dock_bounds_at(
        display(),
        452.0,
        580.0,
        "right",
        12.0,
        Some(a.y),
    );
    assert_eq!(a.y, 40.0);
    assert_eq!(b.y, 40.0);
}
#[test]
fn drag_snaps_to_closest_edge_of_offset_monitor() {
    use agent_orbit::window_state::nearest_dock_side;
    let d = Bounds {
        x: 1440.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    assert_eq!(nearest_dock_side(d, 1500.0, 52.0), "left");
    assert_eq!(nearest_dock_side(d, 3200.0, 52.0), "right");
}
#[test]
fn independent_panel_fits_monitor_without_changing_dock_bounds() {
    use agent_orbit::window_state::panel_bounds;
    let dock = Bounds {
        x: 1390.0,
        y: 100.0,
        width: 50.0,
        height: 440.0,
    };
    let panel = panel_bounds(display(), dock, 360.0, 300.0, "right", 8.0, 50.0);
    assert_eq!(panel.x + panel.width + 8.0, dock.x);
    assert_eq!(panel.y, 0.0);
    let left = panel_bounds(
        display(),
        Bounds { x: 0.0, ..dock },
        360.0,
        1200.0,
        "left",
        8.0,
        300.0,
    );
    assert_eq!(left.x, 58.0);
    assert!(left.height <= 900.0);
}
