# tapd-cli

TAPD 项目管理命令行工具（Rust），**专为 AI Agent 设计**。

通过 Claude Code Skill，AI 可以用自然语言直接操作 TAPD——查需求、建 Bug、统计迭代、记工时，无需记命令。CLI 本身也支持手动使用。

## 快速开始

> **AI Agent 自动安装指引**：如果你是 AI agent 正在帮用户配置此工具，按以下步骤顺序执行即可。

### 第一步：安装 CLI

从 GitHub Releases 下载预编译二进制：

```bash
# macOS (Apple Silicon / Intel)
curl -Lo tapd "https://github.com/nicholasgasior/tapd-cli/releases/latest/download/tapd-macos"
chmod +x tapd
sudo mv tapd /usr/local/bin/

# Linux x86_64
curl -Lo tapd "https://github.com/nicholasgasior/tapd-cli/releases/latest/download/tapd-linux"
chmod +x tapd
sudo mv tapd /usr/local/bin/
```

> 始终安装最新版本。如需指定版本，将 URL 中的 `latest` 替换为版本号（如 `v0.1.8`）。

从源码编译：

```bash
git clone https://github.com/nightlysnow/tapd-cli.git
cd tapd-cli
cargo install --path .
```

验证安装：

```bash
tapd --help
```

### 第二步：安装 Claude Code Skill

**这是关键步骤。** CLI 本身只输出 JSON，安装 Skill 后 Claude Code 才能自动理解 TAPD 相关请求并调用 CLI。

```bash
# 将 skill 复制到 Claude Code skills 目录
cp -r examples/skills/tapd-cli ~/.claude/skills/
```

如果是从 release 下载而非 clone 仓库，可以直接创建：

```bash
mkdir -p ~/.claude/skills/tapd-cli
curl -L "https://raw.githubusercontent.com/nightlysnow/tapd-cli/main/examples/skills/tapd-cli/SKILL.md" \
  -o ~/.claude/skills/tapd-cli/SKILL.md
```

验证安装：

```bash
ls ~/.claude/skills/tapd-cli/SKILL.md
```

### 第三步：配置认证

在**项目根目录**创建 `.env` 文件（或复制模板）：

```bash
cp .env.example .env
```

填入认证信息（二选一）：

| 变量 | 说明 |
|------|------|
| `TAPD_ACCESS_TOKEN` | 个人访问令牌（推荐） |
| `TAPD_API_USER` + `TAPD_API_PASSWORD` | API 账号密码 |

可选配置：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `TAPD_DEFAULT_WORKSPACE_ID` | 默认项目 ID，设置后所有命令可省略 `-w` | — |
| `TAPD_API_BASE_URL` | API 地址 | `https://api.tapd.cn` |
| `CURRENT_USER_NICK` | 当前用户昵称，自动填充 creator 等字段 | 从 Token 自动获取 |
| `BOT_URL` | 企业微信机器人 Webhook URL | — |

验证配置：

```bash
tapd projects
```

如果能返回项目列表，说明配置成功。

### 安装完成

现在可以在 Claude Code 中用自然语言操作 TAPD：

- "帮我看下待办的需求"
- "创一个 bug，标题是首页加载卡顿"
- "统计当前迭代还有多少未关闭的缺陷"
- "给需求 123 记 4 小时工时"

---

## Skill 工作原理

```
用户自然语言 → Claude Code 识别 TAPD 意图 → 触发 tapd-cli skill → 执行 CLI 命令 → 解析 JSON → 返回可读结果
```

**触发关键词**：tapd、TAPD、需求、缺陷、bug、story、迭代、sprint、待办、工时、timesheet、发布计划、工作流，或任何涉及 TAPD 的项目管理操作。

Skill 文件位于 `examples/skills/tapd-cli/SKILL.md`，包含完整的命令映射和常见工作流模板，Claude Code 会据此自动选择正确的命令组合。

---

## CLI 手册

以下是 CLI 的完整使用文档，供手动使用或深入了解。

### 基本语法

```
tapd <命令> [子命令] [选项] [KEY=VALUE ...]
```

- `-w <ID>` — 指定 workspace ID（设置了 `TAPD_DEFAULT_WORKSPACE_ID` 可省略）
- `--raw` — 输出紧凑 JSON，适合管道处理（放在子命令之前）
- 短 ID（≤9 位）自动转为 TAPD 长 ID 格式

### 命令一览

| 命令 | 说明 | 子命令 |
|------|------|--------|
| `projects` | 项目列表 | — |
| `story` | 需求管理 | `list` `create` `update` `count` |
| `task` | 任务管理 | `list` `create` `update` `count` |
| `bug` | 缺陷管理 | `list` `create` `update` `count` |
| `iteration` | 迭代管理 | `list` `create` `update` |
| `comment` | 评论管理 | `list` `create` `update` |
| `wiki` | Wiki 管理 | `list` `create` `update` |
| `tcase` | 测试用例 | `list` `create` |
| `workflow` | 工作流 | `transitions` `status-map` `last-steps` |
| `field` | 字段信息 | `labels` `info` |
| `workspace` | 项目信息 | — |
| `custom-fields` | 自定义字段 | — |
| `todo` | 用户待办 | — |
| `timesheet` | 工时管理 | `list` `add` `update` |
| `release` | 发布计划 | — |
| `attachment` | 附件信息 | — |
| `image` | 图片链接 | — |
| `commit-msg` | SCM 关键字 | — |
| `wecom` | 企业微信消息 | — |
| `relation` | 实体关联 | — |
| `related-bugs` | 需求关联缺陷 | — |
| `update` | 自动更新到最新版本 | — |

### 使用示例

**查询**

```bash
tapd projects                                          # 我参与的项目
tapd story list -w 12345678                            # 需求列表
tapd story list -w 12345678 status=open limit=20       # 带筛选
tapd story count -w 12345678 status=open               # 统计数量
tapd bug list -w 12345678                              # 缺陷列表
tapd iteration list -w 12345678                        # 迭代列表
tapd todo -t story                                     # 用户待办
```

**创建**

```bash
tapd story create -w 12345678 name="新需求" description="描述"
tapd bug create -w 12345678 title="页面崩溃" severity=serious
tapd task create -w 12345678 name="编写测试" story_id=123
```

**更新**

```bash
tapd story update -w 12345678 id=123 status=resolved
tapd bug update -w 12345678 id=456 status=closed resolution=fixed
```

**工时**

```bash
tapd timesheet list -w 12345678 entity_id=123 entity_type=story
tapd timesheet add -w 12345678 entity_id=123 entity_type=story timespent=4h spentdate=2026-05-12
```

**工作流**

```bash
tapd workflow status-map -w 12345678 system=story      # 状态映射
tapd workflow transitions -w 12345678 system=story     # 流转规则
```

**自动更新**

```bash
tapd update                                       # 自动下载并替换为最新版本
```

**管道处理**

```bash
tapd --raw story list -w 12345678 | jq '.data[0]'
tapd --raw bug list -w 12345678 status=open | jq '.[] | {id: .Bug.id, title: .Bug.title}'
```

## 发布流程

本仓库通过 GitHub Actions 自动构建和发布：

1. 推送 `v*` tag 触发 CI
2. 自动编译 macOS (Apple Silicon / Intel) 和 Linux x86_64 二进制
3. 自动创建 GitHub Release 并上传产物
4. 用户可通过 `tapd update` 自动更新到最新版本

## License

MIT
