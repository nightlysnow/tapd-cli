use clap::{Parser, Subcommand};

const WORKSPACE_ENV: &str = "TAPD_DEFAULT_WORKSPACE_ID";

#[derive(Parser)]
#[command(
    name = "tapd",
    bin_name = "tapd",
    about = "TAPD CLI - 高效的 TAPD 项目管理命令行工具",
    version,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,

    /// 输出紧凑 JSON（不格式化）
    #[arg(long, global = true)]
    pub raw: bool,
}

macro_rules! define_actions {
    ($name:ident { $($(#[$m:meta])* $variant:ident),+ $(,)? }) => {
        #[derive(Subcommand)]
        pub enum $name {
            $(
                $(#[$m])*
                $variant {
                    /// 项目 ID
                    #[arg(short = 'w', long, env = WORKSPACE_ENV)]
                    workspace_id: u64,
                    /// 参数 (key=value)
                    #[arg(value_name = "KEY=VALUE")]
                    params: Vec<String>,
                },
            )+
        }
    };
}

#[derive(Subcommand)]
pub enum Cmd {
    /// 获取用户参与的项目列表
    Projects {
        /// 用户昵称（默认使用当前认证用户）
        #[arg(long)]
        nick: Option<String>,
    },

    /// 管理需求
    Story {
        #[command(subcommand)]
        action: StoryAction,
    },

    /// 管理任务
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// 管理缺陷
    Bug {
        #[command(subcommand)]
        action: BugAction,
    },

    /// 管理迭代
    Iteration {
        #[command(subcommand)]
        action: IterAction,
    },

    /// 管理评论
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },

    /// 管理 Wiki
    Wiki {
        #[command(subcommand)]
        action: WikiAction,
    },

    /// 管理测试用例
    Tcase {
        #[command(subcommand)]
        action: TcaseAction,
    },

    /// 工作流信息
    Workflow {
        #[command(subcommand)]
        action: WorkflowAction,
    },

    /// 字段信息
    Field {
        #[command(subcommand)]
        action: FieldAction,
    },

    /// 获取项目信息
    Workspace {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
    },

    /// 获取自定义字段配置
    CustomFields {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 实体类型: stories / tasks / bugs / iterations / tcases
        #[arg(short = 't', long = "type")]
        entity_type: String,
    },

    /// 获取用户待办
    Todo {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: Option<u64>,
        /// 业务类型: story / bug / task
        #[arg(short = 't', long = "type")]
        entity_type: String,
        /// 每页数量
        #[arg(long, default_value = "10")]
        limit: u32,
        /// 页码
        #[arg(long, default_value = "1")]
        page: u32,
    },

    /// 管理花费工时
    Timesheet {
        #[command(subcommand)]
        action: TimesheetAction,
    },

    /// 查询发布计划
    Release {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 参数 (key=value)
        #[arg(value_name = "KEY=VALUE")]
        params: Vec<String>,
    },

    /// 获取附件信息及下载链接
    Attachment {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 参数 (key=value): entry_id=ID type=story|bug
        #[arg(value_name = "KEY=VALUE")]
        params: Vec<String>,
    },

    /// 获取图片下载链接
    Image {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 图片路径
        #[arg(long)]
        image_path: String,
    },

    /// 获取源码提交关键字 (SCM commit message)
    #[command(name = "commit-msg")]
    CommitMsg {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 对象 ID
        #[arg(long)]
        object_id: String,
        /// 对象类型: story / task / bug
        #[arg(short = 't', long = "type")]
        object_type: String,
    },

    /// 发送企业微信消息
    Wecom {
        /// 消息内容 (Markdown 格式)
        #[arg(long)]
        msg: String,
    },

    /// 创建实体关联关系
    Relation {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 参数: source_type=story target_type=bug source_id=ID target_id=ID
        #[arg(value_name = "KEY=VALUE")]
        params: Vec<String>,
    },

    /// 更新 tapd 到最新版本
    #[command(name = "update")]
    Update,

    /// 获取需求关联的缺陷 ID
    RelatedBugs {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
        /// 需求 ID (支持多 ID 逗号分隔)
        #[arg(long)]
        story_id: String,
    },
}

define_actions!(StoryAction {
    /// 查询需求列表
    #[command(alias = "ls")]
    List,
    /// 创建需求
    Create,
    /// 更新需求
    Update,
    /// 获取需求数量
    Count,
});

define_actions!(TaskAction {
    /// 查询任务列表
    #[command(alias = "ls")]
    List,
    /// 创建任务
    Create,
    /// 更新任务
    Update,
    /// 获取任务数量
    Count,
});

define_actions!(BugAction {
    /// 查询缺陷列表
    #[command(alias = "ls")]
    List,
    /// 创建缺陷
    Create,
    /// 更新缺陷
    Update,
    /// 获取缺陷数量
    Count,
});

define_actions!(IterAction {
    /// 查询迭代列表
    #[command(alias = "ls")]
    List,
    /// 创建迭代
    Create,
    /// 更新迭代
    Update,
});

define_actions!(CommentAction {
    /// 查询评论
    #[command(alias = "ls")]
    List,
    /// 创建评论
    Create,
    /// 更新评论
    Update,
});

define_actions!(WikiAction {
    /// 查询 Wiki 列表
    #[command(alias = "ls")]
    List,
    /// 创建 Wiki
    Create,
    /// 更新 Wiki
    Update,
});

define_actions!(TcaseAction {
    /// 查询测试用例
    #[command(alias = "ls")]
    List,
    /// 创建测试用例
    Create,
});

define_actions!(TimesheetAction {
    /// 查询花费工时
    #[command(alias = "ls")]
    List,
    /// 填写花费工时
    Add,
    /// 更新花费工时
    Update,
});

define_actions!(WorkflowAction {
    /// 获取工作流流转细则
    Transitions,
    /// 获取状态中英文映射
    StatusMap,
    /// 获取工作流结束状态
    LastSteps,
});

#[derive(Subcommand)]
pub enum FieldAction {
    /// 获取需求字段中英文映射
    Labels {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
    },
    /// 获取需求字段及候选值
    Info {
        /// 项目 ID
        #[arg(short = 'w', long, env = WORKSPACE_ENV)]
        workspace_id: u64,
    },
}
