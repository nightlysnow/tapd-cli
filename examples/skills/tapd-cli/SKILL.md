---
name: tapd-cli
description: Use the tapd CLI tool to query, create, and manage TAPD project items (stories, bugs, tasks, iterations, wikis, comments, test cases, timesheets, releases). Invoke this skill whenever the user wants to interact with TAPD — searching items, checking status, creating or updating stories/bugs/tasks, viewing backlogs/sprints, logging work hours, querying workflows, or checking release plans. Triggers on "tapd", "TAPD", "需求", "缺陷", "bug", "story", "迭代", "sprint", "待办", "工时", "timesheet", "发布计划", "工作流", or any request involving TAPD project management operations. Also triggers when user mentions a TAPD workspace ID, references tapd.cn URLs, or asks about project items tracked in TAPD.
---

# tapd — TAPD 项目管理 CLI

用 `tapd` 命令操作 TAPD 项目管理平台。输出是 JSON 格式，可以用 `jaq` 做后续处理。

## 前置条件

`tapd` 的认证信息配置在项目目录的 `.env` 文件中。如果遇到认证错误，提示用户检查该文件。

`-w` 参数支持从环境变量 `TAPD_DEFAULT_WORKSPACE_ID` 读取默认值。如果 `.env` 中已配置该变量，所有命令都可以省略 `-w`。

## 命令速览

```
tapd projects                           # 我参与的项目
tapd story   list|create|update|count   # 需求
tapd task    list|create|update|count   # 任务
tapd bug     list|create|update|count   # 缺陷
tapd iteration list|create|update       # 迭代
tapd comment list|create|update         # 评论
tapd wiki    list|create|update         # Wiki
tapd tcase   list|create                # 测试用例
tapd timesheet list|add|update          # 工时
tapd workflow transitions|status-map|last-steps  # 工作流
tapd field   labels|info                # 字段映射
tapd workspace -w <ID>                  # 项目信息
tapd custom-fields -w <ID> -t <type>   # 自定义字段
tapd todo -t <story|bug|task>           # 用户待办
tapd release -w <ID>                    # 发布计划
tapd attachment -w <ID>                 # 附件
tapd image -w <ID> --image-path <path>  # 图片链接
tapd commit-msg -w <ID> --object-id <ID> -t <type>  # 提交关键字
tapd relation -w <ID>                   # 关联关系
tapd related-bugs -w <ID> --story-id <ID>  # 需求关联缺陷
```

## 核心用法

### 参数传递

大多数命令通过 `KEY=VALUE` 形式传递筛选条件或字段值，各参数用空格分隔。含空格的值必须加引号：

```bash
tapd story list status=open priority=high limit=20        # -w 省略，从 TAPD_DEFAULT_WORKSPACE_ID 读取
tapd bug create -w 12345678 title="登录异常" severity=serious  # 也可以显式指定
```

### 全局选项

- `--raw` — 输出紧凑 JSON，适合管道处理。`--raw` 放在子命令之前：
  ```bash
  tapd --raw story list -w <ID> | jaq '.[].Story.name'
  ```

### ID 规则

短 ID（≤9 位）会自动转为 TAPD 长 ID 格式，两种都能用：
```bash
tapd story update -w <ID> id=123 status=resolved      # 短 ID
tapd story update -w <ID> id=1152921131000000123 status=resolved  # 长 ID
```

### 自动填充

创建时 `creator`/`reporter` 会自动填充为配置的用户昵称，无需手动传。

## 常见工作流

### 查看待办

用户说"待办"时，应该查询所有类型（需求 + 缺陷 + 任务），除非用户明确只要某一种：

```bash
tapd todo -t story -w <ID>
tapd todo -t bug -w <ID>
tapd todo -t task -w <ID>
```

用 `jaq` 提取关键字段让结果更可读：
```bash
tapd --raw todo -t story -w <ID> | jaq '[.data[] | {id: .Story.id, name: .Story.name, status: .Story.status, owner: .Story.owner}]'
tapd --raw todo -t bug -w <ID> | jaq '[.data[] | {id: .Bug.id, title: .Bug.title, status: .Bug.status, severity: .Bug.severity}]'
tapd --raw todo -t task -w <ID> | jaq '[.data[] | {id: .Task.id, name: .Task.name, status: .Task.status, owner: .Task.owner}]'
```

### 统计当前迭代缺陷

这是一个多步操作，按以下顺序执行：

1. **找到当前迭代**：查迭代列表，按日期筛选覆盖今天的迭代
   ```bash
   tapd --raw iteration list -w <ID> limit=30 | jaq '[.data[] | .Iteration | {id: .id, name: .name, startdate: .startdate, enddate: .enddate}]'
   ```

2. **查缺陷工作流**：了解哪些状态算"已关闭"
   ```bash
   tapd workflow last-steps -w <ID> system=bug
   ```

3. **按迭代统计**：用 `bug count` 分别查总数和已关闭数
   ```bash
   tapd bug count -w <ID> iteration_id=<迭代ID>
   tapd bug count -w <ID> iteration_id=<迭代ID> status=closed
   ```

### 查询与筛选

```bash
tapd projects                                          # 列出项目
tapd story list -w <ID>                            # 需求列表
tapd story list -w <ID> status=open owner=张三 limit=50  # 带筛选
tapd story count -w <ID> status=open               # 统计数量
tapd bug count -w <ID> severity=serious            # 缺陷数量
tapd iteration list -w <ID> status=open            # 迭代列表
```

### 创建工作项

```bash
tapd story create -w <ID> name="用户登录优化" priority=high description="优化登录流程"
tapd bug create -w <ID> title="页面崩溃" severity=serious module=前端
tapd task create -w <ID> name="编写单元测试" story_id=123 owner=李四
tapd comment create -w <ID> entry_id=123 description="已修复，请复核"
```

**缺陷严重程度**对照：

| 中文 | 值 |
|------|----|
| 致命 | fatal |
| 严重 | serious |
| 一般 | normal |
| 较低 | low |
| 建议 | suggestion |

### 更新状态

更新前建议先查工作流确认合法的状态值：

```bash
tapd workflow status-map -w <ID> system=story      # 状态中英文映射
tapd workflow transitions -w <ID> system=story     # 流转规则
```

然后更新：
```bash
tapd story update -w <ID> id=123 status=resolved
tapd bug update -w <ID> id=456 status=closed resolution=fixed
tapd task update -w <ID> id=789 status=done progress=100
```

### 工时管理

```bash
tapd timesheet list -w <ID> entity_id=123 entity_type=story
tapd timesheet add -w <ID> entity_id=123 entity_type=story timespent=4h spentdate=2026-05-12
```

### 字段与自定义字段

```bash
tapd field labels -w <ID>          # 字段中英文映射
tapd field info -w <ID>            # 字段及候选值
tapd custom-fields -w <ID> -t stories  # 自定义字段配置
```

## 处理 JSON 输出

`tapd` 返回 JSON，用 `jaq` 提取需要的信息：

```bash
tapd --raw story list -w <ID> | jaq '.[].Story.name'
tapd --raw bug list -w <ID> status=open | jaq '.[] | {id: .Bug.id, title: .Bug.title}'
tapd --raw story list -w <ID> | jaq 'group_by(.Story.status) | map({status: .[0].Story.status, count: length})'
```

## 注意事项

- workspace ID (`-w`) 是大多数命令的必需参数，不知道 ID 时先 `tapd projects` 查
- 更新状态前查 `tapd workflow status-map` 确认合法状态值
- 输出是嵌套结构，实体数据在 `Story`/`Bug`/`Task` 等键下面
- `list` 子命令有 `ls` 别名
- `todo` 命令的 workspace ID 是可选的，不传会查所有项目
