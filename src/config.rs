use anyhow::{bail, Result};

pub struct Config {
    pub api_base_url: String,
    pub web_base_url: Option<String>,
    pub access_token: Option<String>,
    pub api_user: Option<String>,
    pub api_password: Option<String>,
    pub bot_url: Option<String>,
    pub nick: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let access_token = Self::env_opt("TAPD_ACCESS_TOKEN");
        let api_user = Self::env_opt("TAPD_API_USER");
        let api_password = Self::env_opt("TAPD_API_PASSWORD");

        if access_token.is_none() && (api_user.is_none() || api_password.is_none()) {
            bail!(
                "认证未配置。请设置环境变量 TAPD_ACCESS_TOKEN，\
                 或同时设置 TAPD_API_USER 和 TAPD_API_PASSWORD"
            );
        }

        Ok(Self {
            api_base_url: Self::env_opt("TAPD_API_BASE_URL")
                .unwrap_or_else(|| "https://api.tapd.cn".into()),
            web_base_url: Self::env_opt("TAPD_BASE_URL"),
            access_token,
            api_user,
            api_password,
            bot_url: Self::env_opt("BOT_URL"),
            nick: Self::env_opt("CURRENT_USER_NICK"),
        })
    }

    pub fn is_cloud(&self) -> bool {
        self.api_base_url.contains("api.tapd.cn")
    }

    fn env_opt(key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.is_empty())
    }
}
