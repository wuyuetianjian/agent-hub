import AppKit
import WebKit

// stdout is reserved for the private parent protocol. No sockets or external listeners.
let maxFrameBytes = 8 * 1024 * 1024
let outputQueue = DispatchQueue(label: "orbit.overlay.output")
func send(_ message: [String: Any]) {
    guard let data = try? JSONSerialization.data(withJSONObject: message), data.count <= maxFrameBytes else { return }
    outputQueue.async {
        FileHandle.standardOutput.write(data + Data([10]))
    }
}
func diagnostic(_ message: String) {
    FileHandle.standardError.write(Data(("[orbit-overlay] " + message + "\n").utf8))
}
func localURL(_ url: URL?) -> Bool {
    guard let url = url else { return false }
    return url.scheme == "orbit-asset" && url.host == "localhost" && url.user == nil && url.password == nil && url.port == nil
}

final class OverlayPanel: NSPanel {
    var allowsKey = false
    override var canBecomeKey: Bool { allowsKey }
    override var canBecomeMain: Bool { false }
}

final class AssetHandler: NSObject, WKURLSchemeHandler {
    private var nextID = 0
    private var tasks: [String: WKURLSchemeTask] = [:]
    private let label: String
    init(label: String) { self.label = label }
    func webView(_ webView: WKWebView, start urlSchemeTask: WKURLSchemeTask) {
        guard localURL(urlSchemeTask.request.url), let url = urlSchemeTask.request.url else {
            urlSchemeTask.didFailWithError(NSError(domain: NSURLErrorDomain, code: NSURLErrorUnsupportedURL))
            return
        }
        nextID += 1
        let id = "\(label):\(nextID)"
        tasks[id] = urlSchemeTask
        send(["type": "asset", "id": id, "path": url.path])
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { [weak self] in
            guard let task = self?.tasks.removeValue(forKey: id) else { return }
            task.didFailWithError(NSError(domain: NSURLErrorDomain, code: NSURLErrorTimedOut))
        }
    }
    func webView(_ webView: WKWebView, stop urlSchemeTask: WKURLSchemeTask) {
        for id in Array(tasks.keys) where tasks[id] === urlSchemeTask { tasks.removeValue(forKey: id) }
    }
    func receive(_ message: [String: Any]) {
        guard let id = message["id"] as? String, let task = tasks.removeValue(forKey: id) else { return }
        guard message["error"] == nil, let encoded = message["body"] as? String,
              let data = Data(base64Encoded: encoded), let url = task.request.url else {
            task.didFailWithError(NSError(domain: NSURLErrorDomain, code: NSURLErrorResourceUnavailable))
            return
        }
        let mime = message["mime"] as? String ?? "application/octet-stream"
        // Match WebKit/Wry custom-protocol responses: HTTP headers preserve
        // MIME/CORS semantics for ES modules. The only allowed origin is ours.
        var headers = ["Content-Type": mime, "Content-Length": String(data.count),
                       "Access-Control-Allow-Origin": "orbit-asset://localhost",
                       "X-Content-Type-Options": "nosniff"]
        let policy = "default-src 'none'; script-src orbit-asset: 'unsafe-inline'; style-src orbit-asset: 'unsafe-inline'; img-src orbit-asset: data: blob:; font-src orbit-asset: data:; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'"
        if mime.lowercased().hasPrefix("text/html") { headers["Content-Security-Policy"] = policy }
        guard let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: headers) else {
            task.didFailWithError(NSError(domain: NSURLErrorDomain, code: NSURLErrorBadServerResponse))
            return
        }
        task.didReceive(response)
        task.didReceive(data)
        task.didFinish()
    }
}

final class Overlay: NSObject, WKScriptMessageHandler, WKNavigationDelegate, WKUIDelegate, NSWindowDelegate {
    let label: String
    let panel: OverlayPanel
    let webView: WKWebView
    let assets: AssetHandler
    private var applyingParentUpdate = false
    private var geometryUpdate: DispatchWorkItem?
    init(label: String, url: URL) {
        self.label = label
        assets = AssetHandler(label: label)
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.setURLSchemeHandler(assets, forURLScheme: "orbit-asset")
        configuration.userContentController.addUserScript(WKUserScript(source: bridgeSource, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        webView = WKWebView(frame: .zero, configuration: configuration)
        panel = OverlayPanel(contentRect: NSRect(x: 0, y: 0, width: 420, height: 480), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        super.init()
        panel.allowsKey = label != "main"
        panel.isReleasedWhenClosed = false
        panel.hidesOnDeactivate = false
        panel.isFloatingPanel = true
        panel.becomesKeyOnlyIfNeeded = false
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
        if #available(macOS 13.0, *) { panel.collectionBehavior.insert(.canJoinAllApplications) }
        panel.level = .screenSaver
        panel.contentView = webView
        panel.delegate = self
        webView.setValue(false, forKey: "drawsBackground")
        webView.navigationDelegate = self
        webView.uiDelegate = self
        configuration.userContentController.add(self, name: "orbit")
        webView.load(URLRequest(url: url))
    }
    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.frameInfo.isMainFrame, localURL(message.frameInfo.request.url),
              let body = message.body as? [String: Any], let id = body["id"] as? NSNumber,
              let command = body["command"] as? String else { return }
        send(["type": "invoke", "label": label, "id": id, "command": command, "args": body["args"] ?? [:]])
    }
    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        decisionHandler(localURL(navigationAction.request.url) && navigationAction.targetFrame != nil ? .allow : .cancel)
    }
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration, for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? { nil }
    private func reportContentError(_ error: Error) {
        let nativeError = error as NSError
        // Policy-denied navigation is intentional, not helper failure.
        if nativeError.domain == NSURLErrorDomain && nativeError.code == NSURLErrorCancelled { return }
        send(["type": "content-error", "label": label,
              "error": String(nativeError.localizedDescription.prefix(512))])
    }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        reportContentError(error)
    }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        reportContentError(error)
    }
    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        // Parent exposes the workspace and shuts down this helper. Do not reload
        // with reset bridge IDs while old parent requests could still complete.
        send(["type": "content-error", "label": label, "error": "Overlay WebContent process terminated"])
    }
    func windowDidMove(_ notification: Notification) { queueGeometry() }
    func windowDidResize(_ notification: Notification) { queueGeometry() }
    private func queueGeometry() {
        guard !applyingParentUpdate else { return }
        geometryUpdate?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self = self, !self.applyingParentUpdate, let screen = NSScreen.screens.first else { return }
            let frame = self.panel.frame
            send(["type": "geometry", "label": self.label, "x": frame.minX,
                  "y": screen.frame.maxY - frame.maxY, "width": frame.width, "height": frame.height])
            self.geometryUpdate = nil
        }
        geometryUpdate = work
        DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(16), execute: work)
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool {
        panel.orderOut(nil)
        send(["type": "closed", "label": label])
        return false
    }
    func receive(_ message: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: message), let json = String(data: data, encoding: .utf8) else { return }
        webView.evaluateJavaScript("window.__ORBIT_RECEIVE__(\(json))", completionHandler: nil)
    }
    func update(_ message: [String: Any]) {
        // Parent geometry has already updated its hidden anchor. Do not echo it
        // back or emit an intermediate size/position during ordered updates.
        geometryUpdate?.cancel()
        geometryUpdate = nil
        applyingParentUpdate = true
        defer { applyingParentUpdate = false }
        let action = message["action"] as? String ?? ""
        let value = message["value"] as? Bool ?? false
        func number(_ key: String) -> CGFloat? {
            guard let n = message[key] as? NSNumber, n.doubleValue.isFinite else { return nil }
            return CGFloat(n.doubleValue)
        }
        switch action {
        case "show": panel.orderFrontRegardless()
        case "hide": panel.orderOut(nil)
        case "focus": if panel.allowsKey { panel.makeKey() }
        case "top": panel.level = value ? .screenSaver : .normal
        case "shadow": panel.hasShadow = value
        case "focusable": panel.allowsKey = label != "main" && value
        case "resizable":
            if value { panel.styleMask.insert(.resizable) } else { panel.styleMask.remove(.resizable) }
        case "position":
            if let x = number("x"), let y = number("y"), let screen = NSScreen.screens.first {
                panel.setFrameOrigin(NSPoint(x: x, y: screen.frame.maxY - y - panel.frame.height))
            }
        case "size":
            if let width = number("width"), let height = number("height"), width > 0, height > 0 {
                let top = panel.frame.maxY
                panel.setContentSize(NSSize(width: width, height: height))
                panel.setFrameOrigin(NSPoint(x: panel.frame.minX, y: top - panel.frame.height))
            }
        case "minSize":
            if let width = number("width"), let height = number("height"), width >= 0, height >= 0 { panel.contentMinSize = NSSize(width: width, height: height) }
        default: break
        }
    }
}

final class Application: NSObject, NSApplicationDelegate {
    var overlays: [String: Overlay] = [:]
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        send(["type": "ready"])
        DispatchQueue.global(qos: .userInitiated).async {
            var buffer = Data()
            while true {
                let chunk = FileHandle.standardInput.availableData
                if chunk.isEmpty { DispatchQueue.main.async { NSApp.terminate(nil) }; return }
                buffer.append(chunk)
                while let newline = buffer.firstIndex(of: 10) {
                    let frame = buffer.prefix(upTo: newline)
                    if frame.count > maxFrameBytes { self.invalidFrame(); return }
                    if let object = try? JSONSerialization.jsonObject(with: frame), let message = object as? [String: Any] {
                        DispatchQueue.main.async { self.receive(message) }
                    } else { self.invalidFrame(); return }
                    buffer.removeSubrange(...newline)
                }
                if buffer.count > maxFrameBytes { self.invalidFrame(); return }
            }
        }
    }
    func invalidFrame() {
        diagnostic("Invalid or oversized parent frame; terminating")
        DispatchQueue.main.async { NSApp.terminate(nil) }
    }
    func receive(_ message: [String: Any]) {
        let label = message["label"] as? String
        switch message["type"] as? String {
        case "create":
            guard let label = label, ["main", "dock-panel", "task-completion"].contains(label), overlays[label] == nil,
                  let address = message["url"] as? String, let url = URL(string: address), localURL(url) else { return }
            overlays[label] = Overlay(label: label, url: url)
        case "window": if let label = label { overlays[label]?.update(message) }
        case "reply": if let label = label { overlays[label]?.receive(message) }
        case "event":
            if let label = label { overlays[label]?.receive(message) }
            else { for overlay in overlays.values { overlay.receive(message) } }
        case "asset": for overlay in overlays.values { overlay.assets.receive(message) }
        case "quit": NSApp.terminate(nil)
        default: break
        }
    }
}

let application = NSApplication.shared
let delegate = Application()
application.delegate = delegate
application.setActivationPolicy(.accessory)
application.run()
