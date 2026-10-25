# Windows PTY test terminal handshake

portable-pty 0.9.0 creates ConPTY with INHERIT_CURSOR. Its startup cursor-position request requires a terminal response before child console initialization. The headless integration test must emulate that response while waiting for READY, including requests split across read chunks. Preserve Unicode round-trip and original account binding assertions. Include captured terminal output on timeout. No production terminal or encoding changes. Validate process tests and formatting locally; Windows verification remains with CI.
