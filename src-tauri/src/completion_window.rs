pub fn completion_visible(ready: bool, has_completions: bool, has_approvals: bool) -> bool {
    ready && has_completions && !has_approvals
}

#[cfg(test)]
mod completion_popup_tests {
    use super::*;

    #[test]
    fn completion_waits_for_approval_and_webview_readiness() {
        assert!(!completion_visible(false, true, false));
        assert!(!completion_visible(true, false, false));
        assert!(!completion_visible(true, true, true));
        assert!(completion_visible(true, true, false));
    }
}
