mod cli;
mod client;
mod config;

use std::collections::HashMap;
use std::path::Path;
use std::process;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;

use cli::*;
use client::*;
use config::Config;

fn main() {
    cleanup_stale_windows_backup();
    if let Err(e) = run() {
        eprintln!("错误: {e:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    if let Cmd::Update { check } = cli.cmd {
        return self_update(check);
    }

    let config = Config::load()?;
    let client = TapdClient::new(&config)?;

    let result = dispatch(&client, &cli)?;
    output(&result, cli.raw)
}

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Windows 自更新时，运行中的旧 exe 被重命名的备份后缀。
/// `write_and_replace` 与 `cleanup_stale_windows_backup` 必须使用同一值，
/// 否则清理逻辑将匹配不到备份文件，故抽成常量避免二者漂移。
#[cfg(windows)]
const WINDOWS_BACKUP_EXT: &str = "exe.old";

/// GitHub Releases 中用于自更新的平台文件名。
fn package_platform() -> Result<&'static str> {
    Ok(match std::env::consts::OS {
        "macos" => "tapd-macos",
        "linux" => "tapd-linux",
        other => bail!("不支持的平台: {other}"),
    })
}

fn self_update(check: bool) -> Result<()> {
    // 超时按最大传输量(二进制下载)预留;版本查询本身很快，不受影响。
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let latest = get_latest_version(&client)?;

    println!("当前版本: v{CURRENT_VERSION}");
    println!("最新版本: v{latest}");

    if latest == CURRENT_VERSION {
        println!("已是最新版本。");
        return Ok(());
    }

    if check {
        println!("有新版本可用，运行 `tapd update` 安装。");
        return Ok(());
    }

    println!("正在下载 v{latest}...");
    let data = download_binary(&client)?;
    replace_binary(&data)?;

    // 由替换后的二进制报告安装版本。
    let reported = std::env::current_exe()
        .ok()
        .and_then(|exe| process::Command::new(exe).arg("--version").output().ok())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("tapd {latest}"));
    println!("更新完成！{reported}");
    Ok(())
}

/// 取 GitHub 最新 release 的版本号。
fn get_latest_version(client: &reqwest::blocking::Client) -> Result<String> {
    let url = "https://api.github.com/repos/nightlysnow/tapd-cli/releases/latest";
    let resp = client
        .get(url)
        .header("User-Agent", "tapd-cli")
        .send()
        .context("请求最新版本失败")?;

    if !resp.status().is_success() {
        bail!("获取最新版本失败: HTTP {}", resp.status());
    }

    let release: Value = resp.json().context("解析 release 响应失败")?;
    let tag = release["tag_name"].as_str().context("未找到 release tag")?;
    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn download_source() -> Result<(String, &'static str)> {
    let platform = package_platform()?;
    Ok((
        format!("https://github.com/nightlysnow/tapd-cli/releases/latest/download/{platform}"),
        "GitHub Releases",
    ))
}

fn download_binary(client: &reqwest::blocking::Client) -> Result<Vec<u8>> {
    let (url, source) = download_source()?;

    eprintln!("正在从 {source} 下载...");

    let resp = client
        .get(&url)
        .send()
        .context("下载失败，请检查网络连接")?;

    if !resp.status().is_success() {
        bail!("下载失败: HTTP {}", resp.status());
    }

    let bytes = resp.bytes().context("读取响应失败")?;

    if bytes.len() < 1_000_000 {
        bail!(
            "下载的文件过小 ({} bytes)，可能不是有效的二进制文件",
            bytes.len()
        );
    }

    Ok(bytes.to_vec())
}

fn replace_binary(data: &[u8]) -> Result<()> {
    let current_exe = std::env::current_exe().context("无法获取当前可执行文件路径")?;
    let dir = current_exe.parent().context("无法获取父目录")?;
    let tmp_path = dir.join(".tapd.update.tmp");

    let result = write_and_replace(data, &tmp_path, &current_exe);
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

fn write_and_replace(data: &[u8], tmp_path: &Path, target: &Path) -> Result<()> {
    std::fs::write(tmp_path, data).context("写入临时文件失败")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp_path, std::fs::Permissions::from_mode(0o755))
            .context("设置可执行权限失败")?;
        std::fs::rename(tmp_path, target).context("替换二进制文件失败")?;
    }

    // Windows 不能覆盖运行中的 exe，但可将其重命名移开：
    // 当前 exe → sidecar(.old)，新文件 → 目标路径，best-effort 清理 .old。
    #[cfg(windows)]
    {
        let old_path = target.with_extension(WINDOWS_BACKUP_EXT);
        // 若上次遗留 .old，先尝试删除（可能仍被占用，忽略错误）
        let _ = std::fs::remove_file(&old_path);
        std::fs::rename(target, &old_path).context("重命名当前程序失败")?;
        // 落盘新版本；若失败，回滚：把旧程序恢复回原位，保证 target 始终存在
        if let Err(e) = std::fs::rename(tmp_path, target) {
            let _ = std::fs::rename(&old_path, target);
            return Err(anyhow::Error::new(e).context("替换二进制文件失败"));
        }
        let _ = std::fs::remove_file(&old_path);
    }

    // 之前此处有一条无条件 rename；拆到 unix/windows 分支后，若目标平台两者都不是，
    // 会跳过替换却仍返回 Ok，导致「更新完成」但二进制未变。用 compile_error 兜底，
    // 避免未来新增平台时静默失效（实际只构建 linux/macOS/windows，均已覆盖）。
    #[cfg(not(any(unix, windows)))]
    compile_error!("write_and_replace: 不支持的目标平台，二进制替换未实现");

    Ok(())
}

/// 启动时清理上次自更新遗留的备份文件。
/// Windows 上运行中的旧 exe 会被重命名为 tapd.exe.old，进程退出后才可删除，
/// 因此下次启动时 best-effort 清理。非 Windows 平台为空操作。
fn cleanup_stale_windows_backup() {
    #[cfg(windows)]
    {
        if let Ok(current_exe) = std::env::current_exe() {
            let old_path = current_exe.with_extension(WINDOWS_BACKUP_EXT);
            let _ = std::fs::remove_file(old_path);
        }
    }
}

fn dispatch(c: &TapdClient, cli: &Cli) -> Result<Value> {
    match &cli.cmd {
        Cmd::Projects { nick } => {
            let nick = nick
                .clone()
                .or_else(|| c.nick.clone())
                .context("需要指定 --nick 或配置 CURRENT_USER_NICK")?;
            c.get(
                "workspaces/user_participant_projects",
                &HashMap::from([("nick".into(), nick)]),
            )
        }

        Cmd::Story { action } => match action {
            StoryAction::List {
                workspace_id,
                params,
            } => do_list(c, "stories", *workspace_id, params),
            StoryAction::Create {
                workspace_id,
                params,
            } => do_create(c, "stories", *workspace_id, params, "creator"),
            StoryAction::Update {
                workspace_id,
                params,
            } => do_update(c, "stories", *workspace_id, params, "current_user"),
            StoryAction::Count {
                workspace_id,
                params,
            } => do_get(c, "stories/count", *workspace_id, params),
        },

        Cmd::Task { action } => match action {
            TaskAction::List {
                workspace_id,
                params,
            } => do_list(c, "tasks", *workspace_id, params),
            TaskAction::Create {
                workspace_id,
                params,
            } => do_create(c, "tasks", *workspace_id, params, "creator"),
            TaskAction::Update {
                workspace_id,
                params,
            } => do_update(c, "tasks", *workspace_id, params, "current_user"),
            TaskAction::Count {
                workspace_id,
                params,
            } => do_get(c, "tasks/count", *workspace_id, params),
        },

        Cmd::Bug { action } => match action {
            BugAction::List {
                workspace_id,
                params,
            } => do_list(c, "bugs", *workspace_id, params),
            BugAction::Create {
                workspace_id,
                params,
            } => do_create(c, "bugs", *workspace_id, params, "reporter"),
            BugAction::Update {
                workspace_id,
                params,
            } => do_update(c, "bugs", *workspace_id, params, "current_user"),
            BugAction::Count {
                workspace_id,
                params,
            } => do_get(c, "bugs/count", *workspace_id, params),
        },

        Cmd::Iteration { action } => match action {
            IterAction::List {
                workspace_id,
                params,
            } => do_list(c, "iterations", *workspace_id, params),
            IterAction::Create {
                workspace_id,
                params,
            } => do_create(c, "iterations", *workspace_id, params, "creator"),
            IterAction::Update {
                workspace_id,
                params,
            } => do_update(c, "iterations", *workspace_id, params, "current_user"),
        },

        Cmd::Comment { action } => match action {
            CommentAction::List {
                workspace_id,
                params,
            } => do_list(c, "comments", *workspace_id, params),
            CommentAction::Create {
                workspace_id,
                params,
            } => c.post("comments", &comment_create_body(c, *workspace_id, params)),
            CommentAction::Update {
                workspace_id,
                params,
            } => do_update(c, "comments", *workspace_id, params, "change_creator"),
        },

        Cmd::Wiki { action } => match action {
            WikiAction::List {
                workspace_id,
                params,
            } => do_list_n(c, "tapd_wikis", *workspace_id, params, "30"),
            WikiAction::Create {
                workspace_id,
                params,
            } => do_create(c, "tapd_wikis", *workspace_id, params, "creator"),
            WikiAction::Update {
                workspace_id,
                params,
            } => do_update(c, "tapd_wikis", *workspace_id, params, "modifier"),
        },

        Cmd::Tcase { action } => match action {
            TcaseAction::List {
                workspace_id,
                params,
            } => do_list_n(c, "tcases", *workspace_id, params, "30"),
            TcaseAction::Create {
                workspace_id,
                params,
            } => do_create(c, "tcases", *workspace_id, params, "creator"),
        },

        Cmd::Workflow { action } => {
            let (ep, ws, params) = match action {
                WorkflowAction::Transitions {
                    workspace_id,
                    params,
                } => ("workflows/all_transitions", workspace_id, params),
                WorkflowAction::StatusMap {
                    workspace_id,
                    params,
                } => ("workflows/status_map", workspace_id, params),
                WorkflowAction::LastSteps {
                    workspace_id,
                    params,
                } => ("workflows/last_steps", workspace_id, params),
            };
            do_get(c, ep, *ws, params)
        }

        Cmd::Field { action } => {
            let (ep, ws) = match action {
                FieldAction::Labels { workspace_id } => ("stories/get_fields_lable", workspace_id),
                FieldAction::Info { workspace_id } => ("stories/get_fields_info", workspace_id),
            };
            do_get(c, ep, *ws, &[])
        }

        Cmd::CustomFields {
            workspace_id,
            entity_type,
        } => {
            let ep = format!("{entity_type}/custom_fields_settings");
            do_get(c, &ep, *workspace_id, &[])
        }

        Cmd::Workspace { workspace_id } => {
            do_get(c, "workspaces/get_workspace_info", *workspace_id, &[])
        }

        Cmd::Todo {
            workspace_id,
            entity_type,
            limit,
            page,
        } => {
            let ep = match entity_type.as_str() {
                "story" => "user_oauth/get_user_todo_story",
                "bug" => "user_oauth/get_user_todo_bug",
                "task" => "user_oauth/get_user_todo_task",
                other => anyhow::bail!("不支持的待办类型: {other} (可选: story/bug/task)"),
            };
            let mut p = HashMap::new();
            if let Some(ws) = workspace_id {
                p.insert("workspace_id".into(), ws.to_string());
            }
            p.insert("limit".into(), limit.to_string());
            p.insert("page".into(), page.to_string());
            c.get(ep, &p)
        }

        Cmd::Timesheet { action } => match action {
            TimesheetAction::List {
                workspace_id,
                params,
            } => do_get(c, "timesheets", *workspace_id, params),
            TimesheetAction::Add {
                workspace_id,
                params,
            }
            | TimesheetAction::Update {
                workspace_id,
                params,
            } => {
                let mut p = with_workspace(*workspace_id, params, &[]);
                c.set_nick_override(&mut p, "owner");
                c.post("timesheets", &to_json(&p))
            }
        },

        Cmd::Release {
            workspace_id,
            params,
        } => do_get(c, "releases", *workspace_id, params),

        Cmd::Attachment {
            workspace_id,
            params,
        } => do_get(c, "attachments", *workspace_id, params),

        Cmd::Image {
            workspace_id,
            image_path,
        } => {
            let p = ws_map(*workspace_id, &[("image_path", image_path)]);
            c.get("files/get_image", &p)
        }

        Cmd::CommitMsg {
            workspace_id,
            object_id,
            object_type,
        } => {
            let mut p = ws_map(
                *workspace_id,
                &[("object_id", object_id), ("type", object_type)],
            );
            c.normalize_ids(&mut p);
            c.get("svn_commits/get_scm_copy_keywords", &p)
        }

        Cmd::Wecom { msg } => c.send_wecom(msg),

        Cmd::Relation {
            workspace_id,
            params,
        } => {
            let p = with_workspace(*workspace_id, params, &[]);
            c.post("relations", &to_json(&p))
        }

        Cmd::RelatedBugs {
            workspace_id,
            story_id,
        } => {
            let mut p = ws_map(*workspace_id, &[("story_id", story_id)]);
            c.normalize_ids(&mut p);
            c.get("stories/get_related_bugs", &p)
        }

        Cmd::Update { .. } => unreachable!("update 已在 run() 中提前处理"),
    }
}

fn ws_map(ws: u64, pairs: &[(&str, &str)]) -> HashMap<String, String> {
    let mut m = HashMap::with_capacity(pairs.len() + 1);
    m.insert("workspace_id".into(), ws.to_string());
    for &(k, v) in pairs {
        m.insert(k.into(), v.into());
    }
    m
}

fn do_list(c: &TapdClient, endpoint: &str, ws: u64, params: &[String]) -> Result<Value> {
    do_list_n(c, endpoint, ws, params, "10")
}

fn do_list_n(
    c: &TapdClient,
    endpoint: &str,
    ws: u64,
    params: &[String],
    limit: &str,
) -> Result<Value> {
    let mut p = with_workspace(ws, params, &[("page", "1"), ("limit", limit)]);
    c.normalize_ids(&mut p);
    c.get(endpoint, &p)
}

fn do_get(c: &TapdClient, endpoint: &str, ws: u64, params: &[String]) -> Result<Value> {
    let p = with_workspace(ws, params, &[]);
    c.get(endpoint, &p)
}

fn do_create(
    c: &TapdClient,
    endpoint: &str,
    ws: u64,
    params: &[String],
    nick_field: &str,
) -> Result<Value> {
    let body = create_body(c, with_workspace(ws, params, &[]), nick_field);
    c.post(endpoint, &body)
}

fn comment_create_body(c: &TapdClient, ws: u64, args: &[String]) -> Value {
    let mut params = with_workspace(ws, args, &[]);
    if let Some(entry_type) = params.get_mut("entry_type") {
        match entry_type.as_str() {
            "story" => *entry_type = "stories".into(),
            "task" => *entry_type = "tasks".into(),
            _ => {}
        }
    }
    create_body(c, params, "author")
}

fn create_body(c: &TapdClient, mut params: HashMap<String, String>, nick_field: &str) -> Value {
    c.normalize_ids(&mut params);
    c.set_nick_default(&mut params, nick_field);
    to_json(&params)
}

fn do_update(
    c: &TapdClient,
    endpoint: &str,
    ws: u64,
    params: &[String],
    nick_field: &str,
) -> Result<Value> {
    let mut p = with_workspace(ws, params, &[]);
    c.normalize_ids(&mut p);
    c.set_nick_override(&mut p, nick_field);
    c.post(endpoint, &to_json(&p))
}

fn output(value: &Value, raw: bool) -> Result<()> {
    let s = if raw {
        serde_json::to_string(value)?
    } else {
        serde_json::to_string_pretty(value)?
    };
    println!("{s}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{comment_create_body, Config, TapdClient};

    #[test]
    fn comment_create_serializes_entry_types() {
        let client = TapdClient::new(&Config {
            api_base_url: "http://127.0.0.1:9".into(),
            web_base_url: None,
            access_token: None,
            api_user: None,
            api_password: None,
            bot_url: None,
            nick: Some("test-author".into()),
        })
        .expect("construct a client with a preset nickname");

        for (input, expected) in [
            (Some("story"), Some("stories")),
            (Some("task"), Some("tasks")),
            (Some("stories"), Some("stories")),
            (Some("tasks"), Some("tasks")),
            (Some("bug"), Some("bug")),
            (Some("bug_remark"), Some("bug_remark")),
            (Some("mini_items"), Some("mini_items")),
            (None, None),
        ] {
            let mut args = vec!["entry_id=123".into(), "description=test comment".into()];
            let mut expected_body = json!({
                "author": "test-author",
                "description": "test comment",
                "entry_id": 1_012_345_678_000_000_123_i64,
                "workspace_id": 12_345_678,
            });
            if let (Some(input), Some(expected)) = (input, expected) {
                args.push(format!("entry_type={input}"));
                expected_body["entry_type"] = json!(expected);
            }

            assert_eq!(
                comment_create_body(&client, 12_345_678, &args),
                expected_body,
                "entry_type input: {input:?}"
            );
        }
    }
}
