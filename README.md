# Agent Orbit

## 中文

### v0.0.1 发布

Windows CLI 启动测试按路径语义比较 Node 入口，兼容正反斜杠分隔符；无界面 PTY 测试模拟 ConPTY 启动所需的光标位置回复。

本次发布将当前开发快照统一标记为 v0.0.1，提供 macOS Intel（x86_64）应用 ZIP 和 SHA-256 校验文件。解压后将 Agent Orbit.app 移入 Applications。此构建未使用 Apple Developer 证书签名或公证；已有 Codex 客户端 Hook 的真实联调仍待验收。

跨平台桌面 Agent 额度与 Token 用量悬浮工具，使用 Tauri 2、React、Rust 和 SQLite。提供普通悬浮窗、macOS 可选灵动岛样式、账号详情、交互终端和确认中心。

- 14 个 Agent 入口及离线图标；点击图标查看账号和全部额度周期。
- 托管 Codex 任务完成通知：Orbit 内以结构化模式启动的 Codex 成功完成一轮任务后显示独立浮框，可关闭或查看原会话；待审批优先，不抢占键盘焦点。失败、中断、手动结束、普通终端及外部客户端不触发成功通知。未读通知按到达顺序显示（最多保留最近 100 条），页面重载不丢失；退出应用后清空。
- 侧边设置：在设置中切换左侧/右侧；`dockSize`、`dockHoverSize`、`dockGap`、`dockPadding`、`dockPanelWidth`、`dockEdgeInset`、`dockHoverDelayMs`、`dockChromeHeight`、`dockPopupHeight` 可通过设置配置覆盖。
- Codex 全窗口审批：在任一已有会话详情选择“接入所有 Codex 窗口”，执行生成的用户级 Hook 安装命令，并在原客户端审查信任。统一入口覆盖同一 Codex 配置目录下不同项目的已有及新开窗口，按原会话分别显示 reason、命令动作和“允许一次／拒绝”，并发请求互不影响；原对话不迁移。
- 每个独立 `CODEX_HOME` 需要安装一次；多账号须配置不同目录，归属不明确时保留原客户端审批。只支持会加载受信任本地 `PermissionRequest` Hook 的客户端，不能保证不支持该事件的窗口接入。安装状态与每个会话收到回调的状态分别显示，不能以安装或模拟测试代替真实验证。
- 全窗口绑定保存在 SQLite；Orbit 重启后更新私有连接信息，无需改变受信任的 Hook 命令。安装器备份配置、保留无关 Hook、迁移已知旧 Orbit 项目 Hook。可暂停所有窗口审批，或单独断开一个会话；断开的会话重启后仍暂停，显式重新接入才恢复。账号配置变更撤销连接和待审批请求。退出 Orbit 后保留原客户端审批，不改变沙箱、原审批策略或持久命令规则。真实 CLI／桌面客户端接入仍待验证，本次 v0.0.1 发布保留此已知限制。
- v0.1.11：审批卡仅展示审批原因与命令／操作，不再显示原始 JSON；浮窗及确认中心共用此显示。未提供原因时明确提示。
- v0.1.10：后台悬浮栏使用原生鼠标检测补足悬停事件，移入圆圈直接显示完整详情，普通点击不再隐式固定；浮窗新增当前账号 Token 时间轴，支持今日按小时及 7 天、30 天、自定义范围。
- v0.1.9：待审批请求由原生窗口层直接展开对应 AI 圆圈旁的确认面板，主窗口关闭到后台时仍可操作；处理完成后清除角标。
- v0.1.8：Codex 默认使用可接收浮窗确认的结构化模式，托管线程显式启用审批策略；终端模式及外部只读历史会话不接管 Codex 确认。详情显示确认通道状态。`codexApprovalPolicy` 默认 `untrusted`（受管会话范围，不修改全局 CLI 配置），可配置为 `on-request`；安全读取等无需审批的操作不会弹窗。
- v0.1.7：主窗口新增关闭（×）按钮，Ctrl/Cmd+W 同样关闭到后台；悬浮栏、监控与会话继续运行。通过托盘、悬浮框入口或 macOS Dock 重新打开主窗口，只有“退出应用”结束进程。
- v0.1.6：主页面新增真实 Token 使用量，支持今日、近 7 天、近 30 天、自定义日期，按小时/天查看趋势与各 AI/账号目录的输入、输出、缓存读取明细。当前读取 Codex、Claude Code、Gemini CLI 本地记录，SQLite 增量去重，未采集显示“—”。悬浮详情恢复鼠标移入直接完整显示，并修复独立窗口主题缺失导致的透明背景。
- v0.1.5：参考 [Agent HUD Simple 模式](https://github.com/jazzenchen/agent-hud-open)，采用安静的贴边面板、悬停精简摘要、点击固定完整卡片三级展示；待确认请求直接展开操作卡。新增真实会话状态与会话快捷入口，历史记录不会显示成实时运行；额度百分比继续只在悬停内容中显示。
- v0.1.4：侧边圆环不再常驻显示百分比，数字只在悬停详情中显示；仅当前图标放大，整列位置保持稳定。详情弹层独立，点击可固定，关闭或 Esc 取消；默认紧贴屏幕边缘。拖动松手后吸附，支持位置锁定和按左右边缘记忆位置。大小设置提供预设与预览。任务栏/Dock 和托盘入口只打开主窗口，悬浮栏及详情不单独占用任务栏。
- v0.1.3：悬浮框与主界面/设置分为独立窗口，关闭主界面保留悬浮框；设置中的收起大小（默认 32px）与悬停大小（默认 44px）滑块立即生效并保存，移出后自动缩小。拖动顶部把手后自动贴靠最近的左右屏幕边缘；旧版默认 60px 自动迁移到小尺寸。
- v0.1.2：默认采用左右侧边竖排圆环，中心显示 AI 图标、外圈显示剩余额度比例；悬停展开详情，交互确认在对应 AI 旁弹出。支持拖动纵向位置，切换详情保留未提交回答。未知额度显示“—”。
- v0.1.1：支持 NVM/Homebrew/桌面内置 CLI 探测，以及 Codex CLI/桌面端已有会话记录；会话入口和账号详情显示相关记录。
- 摘要默认显示最短周期的剩余额度，例如优先显示 5 小时额度；也可选择剩余比例最低的周期。
- Codex、Claude Code、Gemini CLI、GitHub Copilot 已实现内置额度读取路径；账号资格和凭据格式会影响可用性。
- 其余 Agent 提供发现、配置、CLI 会话、官方用量入口及自定义 JSON 采集器；**尚未实现它们全部的原生额度接口**。
- Codex 支持结构化确认/问答卡片；Claude 托管终端支持权限和问答 Hook；其他 CLI 可在内置终端交互。
- 中文/English、深浅色主题、托盘、置顶、开机启动、多个账号配置和本地持久化。

当前为可构建的开发版。原生启动在当前环境的框架初始化阶段中止，尚未完成启动验收。macOS 原生构建已在本机运行编译检查；Windows/Linux 的 CI 构建流程已配置，实际系统运行和全部真实账号联调尚未完成。详细状态见 [接入说明](docs/providers.md) 与 [验证记录](docs/validation.md)。

### 开发运行

需要 Node.js 22.6+、npm、Rust stable 及 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。macOS 需要 Xcode Command Line Tools，Windows 需要 MSVC Build Tools 和 WebView2，Linux 需要 WebKitGTK 4.1 开发库。

```sh
npm ci
npm run desktop
```

浏览器预览：`npm run dev`。预览不会读取本机账号、启动 CLI 或显示模拟余额。

```sh
npm test
make test
npm run build
npm run desktop:build
```

Windows 没有 Make 时，分别运行 `npm test` 和 `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`。打包产物位于 `src-tauri/target/release/bundle/`；`--debug` 构建位于 `target/debug/bundle/`。

设置中的“显示模式”可切换 macOS 灵动岛。账号编辑器内的参数采用 JSON 字符串数组。配置优先级为数据库、进程配置、外置文件；运行时配置可通过 `AGENT_ORBIT_CONFIG` 和 `AGENT_ORBIT_DATA_DIR` 指定。[配置说明](docs/configuration.md) 包含采集方式与数据位置。

### 接入范围

Claude Code、Codex、Gemini CLI、Google Antigravity、Cursor、GitHub Copilot、Windsurf、Cline、Roo Code、OpenCode、Continue、Kiro、Amazon Q Developer、Aider。

额度未知、登录失效、无权限和接口不支持分别展示。Gemini 个人订阅迁移与内部用量接口可能影响额度读取，详见 [Provider 状态](docs/providers.md)。普通外部终端不会被自动接管。

## English

### v0.0.1 release

Windows CLI launcher tests compare Node entry points as paths, accepting equivalent slash and backslash separators. Headless PTY tests answer the cursor-position query required by ConPTY startup.

This release labels the current development snapshot v0.0.1 and provides a macOS Intel (x86_64) application ZIP and SHA-256 checksums. Extract and move Agent Orbit.app into Applications. The build is not signed with an Apple Developer certificate or notarized; live integration of the existing Codex client hook remains unverified.

A desktop agent quota companion built with Tauri 2, React, Rust and SQLite. It includes a floating widget, an optional macOS Dynamic Island-style layout, account details, interactive terminals and an approval inbox.

- Fourteen agent entries with offline icons; click an icon for account details and all quota windows.
- Managed Codex task completion notices: successful structured turns started inside Orbit open an independent, non-activating notification with Dismiss and View session actions. Pending approvals take priority. Failed/interrupted/stopped turns, terminal sessions and external clients do not announce success. The in-memory FIFO keeps the latest 100 unread notices across webview reloads, but not app restarts.
- Dock configuration: choose left/right in Settings; override `dockSize`, `dockHoverSize`, `dockGap`, `dockPadding`, `dockPanelWidth`, `dockEdgeInset`, `dockHoverDelayMs`, `dockChromeHeight` and `dockPopupHeight` through settings configuration.
- All-window Codex approvals: choose “Connect all Codex windows” from any existing session, run the generated user-level hook installer, then review/trust it in the original client. One dispatcher covers existing and new windows across projects sharing that Codex home. Each request displays its original session/project, reason, action and one-time allow/deny choices; concurrent requests stay isolated and conversations remain in their original clients.
- Install once per distinct `CODEX_HOME`. Separate account profiles need distinct homes; ambiguous ownership falls back to the original approval UI. Clients must support and load trusted local `PermissionRequest` hooks; unsupported clients cannot be connected by this hook. Installation coverage is shown separately from each session's actual callback status. Neither installation nor synthetic tests prove real client integration.
- Registrations live in SQLite. Orbit refreshes private listener descriptors on restart without changing the trusted hook command. Installation backs up existing configuration, preserves unrelated hooks and migrates known old Orbit project handlers. Pause all windows or disconnect one session; session disconnects persist across restarts until explicitly reconnected. Account edits revoke connections and pending approvals. When Orbit is offline, the original client retains approval control. Sandbox policy and persistent command rules remain unchanged. Genuine CLI/desktop client integration still needs verification; v0.0.1 ships with this known limitation.
- v0.1.11: Approval cards show only the reason and command/action instead of raw JSON, in both floating panels and the approval center. Missing reasons are identified explicitly.
- v0.1.10: Native cursor sampling supports hover on inactive floating windows. Entering a circle opens full details, and ordinary clicks no longer pin implicitly. Floating details include the selected account’s token timeline with hourly Today and 7-day, 30-day, and custom ranges.
- v0.1.9: New approval requests open the corresponding AI circle’s independent confirmation panel from the native window layer, including while the workspace is hidden. Resolving a request clears its badge.
- v0.1.8 defaults Codex launches to structured mode and explicitly sets approval policy on managed threads. Terminal mode and imported read-only history do not route Codex approvals to the HUD. Details show actual channel connectivity. `codexApprovalPolicy` defaults to `untrusted` and can use `on-request`; this applies only to managed sessions and does not modify global CLI config. Operations that do not require approval will not prompt.
- v0.1.7 adds an explicit close (×) button and Ctrl/Cmd+W to hide the workspace while the HUD, monitoring and sessions keep running. Reopen via the tray, HUD settings or macOS Dock; only Quit application ends the process.
- v0.1.6 adds real recorded token usage to Overview: Today, Last 7/30 days and custom dates, hourly/daily trends and per-agent/account input, output and cache-read breakdowns. Codex, Claude Code and Gemini CLI local logs are ingested incrementally into SQLite with deduplication; unavailable usage shows “—”. Full floating details now open directly on hover, with the detached theme/background fixed.
- v0.1.5 follows the interaction hierarchy of [Agent HUD Simple mode](https://github.com/jazzenchen/agent-hud-open): a quiet edge rail, compact hover peek, and click-pinned full card. Pending confirmations bypass the peek. Live managed-session indicators and session shortcuts are shown without treating imported history as live activity. Percentage labels remain confined to hover content.
- v0.1.4 removes persistent percentage badges from orbs; percentages appear in hover details. Only the targeted icon magnifies within fixed cells. Details use an independent window with click-to-pin and Close/Escape dismissal. The dock defaults to the physical screen edge, snaps on drag release, supports locking and remembers placement per side. Size settings include presets and previews. OS taskbar/Dock and tray activation open only the workspace; floating windows have no separate taskbar entries.
- v0.1.3 separates the dock and workspace/settings into independent windows. Closing the workspace leaves the dock visible. Resting (default 32px) and hover (default 44px) size sliders apply and save immediately; leaving the dock shrinks it. Drag the top handle to snap to the nearest left/right screen edge. The legacy 60px default migrates to the smaller resting size.
- v0.1.2 adds a vertical side dock with AI icons, remaining-quota rings, hover details and confirmations beside the originating AI. Vertical position and unsent answers survive detail switches; unknown quota shows “—”.
- v0.1.1 adds NVM/Homebrew/bundled-CLI discovery and read-only Codex CLI/desktop session records, accessible from the widget and agent details.
- Summary defaults to the shortest quota period, such as the five-hour allowance. Lowest-known-remaining is also selectable.
- Built-in quota paths are implemented for Codex, Claude Code, Gemini CLI and GitHub Copilot; account eligibility and credential formats determine availability.
- Other entries support discovery, configuration, generic CLI sessions, official usage-page links and custom JSON collectors. **Their native account quota APIs are not all implemented.**
- Structured approval/question cards for Codex, managed Claude hooks, and direct terminal interaction for other CLIs.
- Chinese/English, light/dark themes, tray restore, pinning, launch at login, multiple profiles and local persistence.

This is a buildable development version. Native startup aborted during framework initialization in this environment and has not passed runtime acceptance. The native macOS build was compiled locally. Windows/Linux CI jobs are configured; actual OS runtime checks and all live-account integrations are not yet verified. See [provider status](docs/providers.md) and [validation](docs/validation.md).

### Run and build

Install Node.js 22.6+, npm, Rust stable and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), then run:

```sh
npm ci
npm run desktop
```

`npm run dev` opens a browser preview without local-account access, CLI execution or simulated balances. `npm test` runs frontend tests, `make test` also runs core Rust tests, `npm run build` builds the frontend, and `npm run desktop:build` packages the native application. On Windows without Make, run the Cargo test command shown in the Chinese section directly. Native bundles are generated under `src-tauri/target/release/bundle/` (or `debug/bundle/` for debug builds).

Choose the macOS layout in Settings. Configuration precedence is database, process configuration, external files, then bundled defaults. `AGENT_ORBIT_CONFIG` and `AGENT_ORBIT_DATA_DIR` allow externally managed configuration/data locations. See [configuration](docs/configuration.md) for details and [provider status](docs/providers.md) for collector and CLI setup.

Token 历史配置 / Token history configuration: `usageRefreshSeconds` (60), `usageHistoryDays` (90, up to 366), `usageMaxFiles` (5000), `usageMaxBytesPerScan` (32 MiB), `usageMaxLineBytes` (4 MiB per JSONL line), `usageMaxBuckets` (2200). These use the existing database → service configuration → external config defaults priority. Provider `usageLogPaths` and `usageFormat` configure local log sources. 仅保存时间、标识及计数，不保存对话正文；统计范围为已采集的本地记录，不能代表所有设备或官方账单。Only timestamps, identifiers and counts are stored, without conversation content; local recorded usage does not represent every device or official billing.
