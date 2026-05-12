# CLAUDE.md

## 项目概述

tapd-cli — TAPD 项目管理命令行工具（Rust），专为 AI Agent 设计。通过 Claude Code Skill 实现自然语言操作 TAPD。

- 当前版本：0.1.8
- 二进制名：`tapd`
- 仓库：`github.com/nightlysnow/tapd-cli`

## 构建

```bash
# 本地开发
cargo build

# Release 构建
cargo build --release
```

## 发布

1. 更新 `Cargo.toml` 和 `Cargo.lock` 版本号
2. 推送 tag：`git tag v0.x.x && git push origin v0.x.x`
3. GitHub Actions 自动编译并创建 Release

## 项目结构

```
src/
├── cli.rs      # clap 命令行定义
├── client.rs   # TAPD API 客户端
├── config.rs   # 配置加载（.env）
└── main.rs     # 入口 + self-update 逻辑
scripts/
└── release-macos.sh  # macOS Universal Binary 构建+上传脚本
```

## 认证配置

通过 `.env` 文件配置：
- `TAPD_ACCESS_TOKEN`（推荐）或 `TAPD_API_USER` + `TAPD_API_PASSWORD`
- `TAPD_DEFAULT_WORKSPACE_ID` — 默认项目 ID
- `CURRENT_USER_NICK` — 用户昵称（可从 Token 自动获取）
