mod cli;
mod client;
mod config;

use std::collections::HashMap;
use std::process;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;

use cli::*;
use client::*;
use config::Config;

fn main() {
    if let Err(e) = run() {
        eprintln!("错误: {e:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    if matches!(cli.cmd, Cmd::Update) {
        return self_update();
    }

    let config = Config::load()?;
    let client = TapdClient::new(&config)?;

    let result = dispatch(&client, &cli)?;
    output(&result, cli.raw)
}

fn self_update() -> Result<()> {
    let platform = match std::env::consts::OS {
        "macos" => "tapd-macos",
        "linux" => "tapd-linux",
        _ => bail!("不支持的平台: {}，仅支持 macOS 和 Linux", std::env::consts::OS),
    };

    let url = format!(
        "https://github.com/nightlysnow/tapd-cli/releases/latest/download/{platform}"
    );
    let source = "GitHub Releases";

    eprintln!("正在从 {source} 下载最新版本...");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("创建 HTTP 客户端失败")?;
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

    let current_exe = std::env::current_exe().context("无法获取当前可执行文件路径")?;
    let tmp_path = current_exe.with_extension("new");

    let install = || -> Result<()> {
        std::fs::write(&tmp_path, &bytes).context("写入临时文件失败")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
                .context("设置可执行权限失败")?;
        }

        std::fs::rename(&tmp_path, &current_exe).context("替换二进制文件失败")?;
        Ok(())
    };

    if let Err(e) = install() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    let version = process::Command::new(&current_exe)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let version = version.trim();

    eprintln!("更新成功！{version}");
    Ok(())
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
            } => do_create(c, "comments", *workspace_id, params, "author"),
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

        Cmd::Update => unreachable!("update 已在 run() 中提前处理"),
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
    let mut p = with_workspace(ws, params, &[]);
    c.normalize_ids(&mut p);
    c.set_nick_default(&mut p, nick_field);
    c.post(endpoint, &to_json(&p))
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
