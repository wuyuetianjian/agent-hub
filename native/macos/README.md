# macOS overlay helper

`main.swift` is a separate Accessory process owning genuine nonactivating
`NSPanel` instances. The regular Tauri process retains the workspace, Dock icon,
and Cmd-Tab identity. `build.rs` compiles this source plus an embedded `bridge.js`
on macOS and declares the target-suffixed executable through Tauri `externalBin`.
Tauri copies the development sidecar beside the executable and bundles/signs it
under `Contents/MacOS` for release. Xcode Command Line Tools with a macOS 13+ SDK
are required to compile; the deployment target remains macOS 11.

The parent launches the helper with private stdin/stdout pipes. Each line is a
JSON object, at most 8 MiB. stdout is protocol-only, stderr is diagnostics; EOF or
invalid/oversized input terminates the helper. No port, socket, or public bridge
endpoint exists. `quit` terminates it explicitly.

Assets are requested as `{type:"asset",id:"label:counter",path:"/index.html"}`;
parent responses echo the string ID with `mime` and a base64 `body`, or `error`.
Requests expire after 30 seconds and WebKit cancellation removes the task.
Only `orbit-asset://localhost` is navigable. HTTP-style responses retain module MIME/CORS semantics with only the local origin
allowed. HTML receives a restrictive CSP response header before parsing, denying network connections, remote scripts, frames, and forms.
Bridge messages must originate in the main frame at that local origin. The
native owner stamps its immutable label on every invoke. Replies use numeric
invoke IDs; JS promises expire after 30 seconds except human-paced directory selection;
page teardown rejects all pending requests and clears timers/listeners. Events carry `{event,payload}`.

The collection behavior and level follow Apple's DTS guidance:
https://developer.apple.com/forums/thread/826308
`canJoinAllApplications` is availability-guarded at macOS 13. This does not
promise drawing above the login/lock screen, exclusive/protected video, or all
third-party presentation modes. Showing a panel orders it front without app
activation; only detail/completion panels can become key.

## Verification

`node --import tsx --test tests/macos-helper.test.ts` exercises the actual JS
bridge (reply/error pairing, events/unlisten, timeout) and source-level native
invariants. These tests do **not** establish AppKit compilation or behavior.
A macOS build must compile Swift and test the packaged signed sidecar. Manual
acceptance additionally needs another app's native fullscreen Space, focus
retention while opening details, text entry, multiple displays/scales, Spaces
switching, and main app Dock/Cmd-Tab identity.
