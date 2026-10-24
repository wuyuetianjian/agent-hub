# Desktop Agent Widget

## 中文

面向 macOS、Windows 和 Linux 的桌面 Agent 额度悬浮工具，计划汇总已登录账号的额度，并集中处理 CLI 确认和问答。主流 Agent 使用独立图标和可扩展适配器。

**当前状态：需求与设计草案。尚未实现可运行应用或完成账号接入。**

首批拟覆盖 Claude Code、Codex、Gemini CLI、Google Antigravity、Cursor、GitHub Copilot、Windsurf、Cline、Roo Code、OpenCode、Continue、Kiro、Amazon Q Developer 和 Aider。各产品的订阅额度、API 用量和交互能力需分别验证。

阅读 [需求与设计草案](docs/superpowers/specs/2026-10-24-desktop-agent-widget-design.md)。推荐技术栈为 Tauri 2、React/TypeScript、Rust 和 SQLite。运行时设置优先存入数据库，其次使用进程配置和可外置的配置文件。

## English

A planned desktop widget for macOS, Windows, and Linux that monitors signed-in agent accounts and brings CLI approvals and questions into a floating window. Each agent has its own icon and an extensible provider adapter.

**Current status: requirements and design draft. No runnable application or account integrations have been implemented.**

The initial proposed catalog includes Claude Code, Codex, Gemini CLI, Google Antigravity, Cursor, GitHub Copilot, Windsurf, Cline, Roo Code, OpenCode, Continue, Kiro, Amazon Q Developer, and Aider. Subscription quotas, API usage, and interaction capabilities require separate verification for each product.

See the [requirements and design draft](docs/superpowers/specs/2026-10-24-desktop-agent-widget-design.md). The recommended stack is Tauri 2, React/TypeScript, Rust, and SQLite. Configuration precedence is database settings, process configuration, then externally replaceable configuration files.
