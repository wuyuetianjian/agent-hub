# Windows npm shim test portability

The Windows CI launch test compares strings with different path separators for the same Node entry point. Compare Path values so equivalent Windows separators are accepted, while a different target file or modified shell-sensitive argument still fails. Keep production launcher behavior unchanged. Validate the focused launch tests and Rust formatting locally; Windows confirmation requires CI.
