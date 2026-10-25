# Agent Orbit

## 中文

跨平台桌面 Agent 额度悬浮工具，使用 Tauri 2、React、Rust 和 SQLite。提供普通悬浮窗、macOS 可选灵动岛样式、账号详情、交互终端和确认中心。

- 14 个 Agent 入口及离线图标；点击图标查看账号和全部额度周期。
- 摘要默认显示最短周期的剩余额度，例如优先显示 5 小时额度；也可选择剩余比例最低的周期。
- Codex、Claude Code、Gemini CLI、GitHub Copilot 已实现内置额度读取路径；账号资格和凭据格式会影响可用性。
- 其余 Agent 提供发现、配置、CLI 会话、官方用量入口及自定义 JSON 采集器；**尚未实现它们全部的原生额度接口**。
- Codex 支持结构化确认/问答卡片；Claude 托管终端支持权限和问答 Hook；其他 CLI 可在内置终端交互。
- 中文/English、深浅色主题、托盘、置顶、开机启动、多个账号配置和本地持久化。

当前为可构建的开发版。macOS 原生构建已在本机运行编译检查；Windows/Linux 的 CI 构建流程已配置，实际系统运行和全部真实账号联调尚未完成。详细状态见 [接入说明](docs/providers.md) 与 [验证记录](docs/validation.md)。

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
- Summary defaults to the shortest quota period, such as the five-hour allowance. Lowest-known-remaining is also selectable.
- Built-in quota paths are implemented for Codex, Claude Code, Gemini CLI and GitHub Copilot; account eligibility and credential formats determine availability.
- Other entries support discovery, configuration, generic CLI sessions, official usage-page links and custom JSON collectors. **Their native account quota APIs are not all implemented.**
- Structured approval/question cards for Codex, managed Claude hooks, and direct terminal interaction for other CLIs.
- Chinese/English, light/dark themes, tray restore, pinning, launch at login, multiple profiles and local persistence.

This is a buildable development version. The native macOS build was compiled locally. Windows/Linux CI jobs are configured; actual OS runtime checks and all live-account integrations are not yet verified. See [provider status](docs/providers.md) and [validation](docs/validation.md).

### Run and build

Install Node.js 22.6+, npm, Rust stable and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), then run:

```sh
npm ci
npm run desktop
```

`npm run dev` opens a browser preview without local-account access, CLI execution or simulated balances. `npm test` runs frontend tests, `make test` also runs core Rust tests, `npm run build` builds the frontend, and `npm run desktop:build` packages the native application. On Windows without Make, run the Cargo test command shown in the Chinese section directly. Native bundles are generated under `src-tauri/target/release/bundle/` (or `debug/bundle/` for debug builds).

Choose the macOS layout in Settings. Configuration precedence is database, process configuration, external files, then bundled defaults. `AGENT_ORBIT_CONFIG` and `AGENT_ORBIT_DATA_DIR` allow externally managed configuration/data locations. See [configuration](docs/configuration.md) for details and [provider status](docs/providers.md) for collector and CLI setup.
