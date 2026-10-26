//! Shared routing policy: workspace always belongs to the regular application.
pub fn is_overlay(label: &str) -> bool {
    matches!(label, "main" | "dock-panel" | "task-completion")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_floating_windows_belong_to_helper() {
        for label in ["main", "dock-panel", "task-completion"] {
            assert!(is_overlay(label), "{label} must use the accessory helper");
        }
        for label in ["workspace", "", "unknown", "main/../workspace"] {
            assert!(!is_overlay(label));
        }
    }
}
