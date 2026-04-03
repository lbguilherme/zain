use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::{CdpError, Result};

pub struct ChromiumProcess {
    child: Child,
    pub ws_url: String,
    pub debug_port: u16,
    temp_user_data_dir: Option<PathBuf>,
    /// Held to limit concurrent browser instances. Dropped with the process.
    pub(crate) _launch_permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

pub struct LaunchOptions {
    pub executable: String,
    pub port: u16,
    pub headless: bool,
    pub user_data_dir: Option<String>,
    pub window_size: (u32, u32),
    pub extra_args: Vec<String>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            executable: detect_executable(),
            port: 0,
            headless: true,
            user_data_dir: None,
            window_size: (1920, 1080),
            extra_args: Vec::new(),
        }
    }
}

fn detect_executable() -> String {
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ];

    for candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            return candidate.into();
        }
        if !candidate.contains('/')
            && let Ok(output) = std::process::Command::new("which").arg(candidate).output()
            && output.status.success()
        {
            return candidate.into();
        }
    }

    "chromium".into()
}

fn random_hex() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h = s.build_hasher();
    h.write_u64(std::process::id() as u64);
    let a = h.finish();
    let mut h = s.build_hasher();
    h.write_u64(a);
    let b = h.finish();
    format!("{:016x}{:016x}", a, b)
}

impl ChromiumProcess {
    pub async fn launch(opts: LaunchOptions) -> Result<Self> {
        let port = if opts.port == 0 {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").map_err(CdpError::ProcessStart)?;
            listener
                .local_addr()
                .map_err(CdpError::ProcessStart)?
                .port()
        } else {
            opts.port
        };

        let mut args = vec![
            format!("--remote-debugging-port={}", port),
            format!(
                "--window-size={},{}",
                opts.window_size.0, opts.window_size.1
            ),
            "--disable-blink-features=AutomationControlled".into(),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-background-networking".into(),
            "--disable-sync".into(),
            "--disable-translate".into(),
            "--metrics-recording-only".into(),
            "--mute-audio".into(),
            "--no-sandbox".into(),
        ];

        if opts.headless {
            args.push("--headless=new".into());
        }

        let temp_user_data_dir = if let Some(ref dir) = opts.user_data_dir {
            args.push(format!("--user-data-dir={}", dir));
            None
        } else {
            let tmp = PathBuf::from("/tmp").join(format!("chromium-driver-{}", random_hex()));
            args.push(format!("--user-data-dir={}", tmp.display()));
            Some(tmp)
        };

        args.extend(opts.extra_args);

        let mut child = Command::new(&opts.executable)
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(CdpError::ProcessStart)?;

        let stderr = child.stderr.take().expect("stderr was piped");
        let mut reader = BufReader::new(stderr).lines();

        let ws_url = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(url) = line.strip_prefix("DevTools listening on ") {
                    return Ok(url.trim().to_owned());
                }
            }
            Err(CdpError::BrowserCrashed)
        })
        .await
        .map_err(|_| CdpError::Timeout(std::time::Duration::from_secs(10)))??;

        Ok(Self {
            child,
            ws_url,
            debug_port: port,
            temp_user_data_dir,
            _launch_permit: None,
        })
    }

    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await.map_err(CdpError::ProcessStart)?;
        self.cleanup_temp_dir();
        Ok(())
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.child.wait().await.map_err(CdpError::ProcessStart)?;
        self.cleanup_temp_dir();
        Ok(status)
    }

    fn cleanup_temp_dir(&mut self) {
        if let Some(dir) = self.temp_user_data_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

impl Drop for ChromiumProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        self.cleanup_temp_dir();
    }
}
