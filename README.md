# Agent Orbit

## 中文

跨平台桌面 Agent 额度与 Token 用量悬浮工具，使用 Tauri 2、React、Rust 和 SQLite。提供普通悬浮窗、macOS 可选灵动岛样式、账号详情、交互终端和确认中心。

- 14 个 Agent 入口及离线图标；点击图标查看账号和全部额度周期。
- 侧边设置：在设置中切换左侧/右侧；`dockSize`、`dockHoverSize`、`dockGap`、`dockPadding`、`dockPanelWidth`、`dockEdgeInset`、`dockHoverDelayMs`、`dockChromeHeight`、`dockPopupHeight` 可通过设置配置覆盖。
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

A desktop agent quota companion built with Tauri 2, React, Rust and SQLite. It includes a floating widget, an optional macOS Dynamic Island-style layout, account details, interactive terminals and an approval inbox.

- Fourteen agent entries with offline icons; click an icon for account details and all quota windows.
- Dock configuration: choose left/right in Settings; override `dockSize`, `dockHoverSize`, `dockGap`, `dockPadding`, `dockPanelWidth`, `dockEdgeInset`, `dockHoverDelayMs`, `dockChromeHeight` and `dockPopupHeight` through settings configuration.
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
