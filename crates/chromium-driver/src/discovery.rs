use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "User-Agent")]
    pub user_agent: String,
    #[serde(rename = "V8-Version")]
    pub v8_version: String,
    #[serde(rename = "WebKit-Version")]
    pub webkit_version: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: String,
}

pub async fn get_version(host: &str, port: u16) -> Result<VersionInfo> {
    let url = format!("http://{}:{}/json/version", host, port);
    let info = reqwest::get(&url).await?.json().await?;
    Ok(info)
}
