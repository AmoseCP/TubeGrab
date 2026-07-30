//! 结构化错误：前端根据 kind 决定提示方式（engine 类错误附带"更新引擎"入口）。

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiError {
    /// "invalid_url" | "network" | "engine" | "internal"
    pub kind: String,
    pub message: String,
}

impl ApiError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }
    pub fn invalid_url(msg: impl Into<String>) -> Self {
        Self::new("invalid_url", msg)
    }
    pub fn network(msg: impl Into<String>) -> Self {
        Self::new("network", msg)
    }
    pub fn engine(msg: impl Into<String>) -> Self {
        Self::new("engine", msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new("internal", msg)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

/// 根据 yt-dlp stderr 内容归类为对用户友好的错误。
pub fn classify_ytdlp_error(stderr: &str) -> ApiError {
    let tail: String = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let lower = stderr.to_lowercase();
    if lower.contains("unsupported url") || lower.contains("is not a valid url") {
        ApiError::invalid_url("链接无效或不受支持，请检查后重试")
    } else if lower.contains("unable to download")
        && (lower.contains("getaddrinfo") || lower.contains("timed out") || lower.contains("connection"))
        || lower.contains("network")
        || lower.contains("getaddrinfo failed")
    {
        ApiError::network(format!("网络连接失败，请检查网络后重试\n{tail}"))
    } else if lower.contains("sign in") || lower.contains("login") || lower.contains("age") {
        ApiError::new("unsupported", "该视频需要登录或有年龄限制，v1 不支持此类内容")
    } else {
        ApiError::engine(format!(
            "解析/下载失败，下载引擎可能已过期，请尝试在设置中更新引擎。\n详细信息:\n{tail}"
        ))
    }
}
