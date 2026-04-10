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
    /// On Linux, holds the Xvfb child process. Killed alongside the
    /// Chromium child.
    #[cfg(target_os = "linux")]
    xvfb_child: Option<Child>,
    /// Held to limit concurrent browser instances. Dropped with the process.
    pub(crate) _launch_permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

#[derive(Clone)]
pub struct LaunchOptions {
    pub executable: String,
    pub port: u16,
    pub user_data_dir: Option<String>,
    pub window_size: (u32, u32),
    pub extra_args: Vec<String>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            executable: detect_executable(),
            port: 0,
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

        // On Linux, always run Chromium inside Xvfb so it works without a
        // real display. On other platforms the browser window is visible.
        #[cfg(target_os = "linux")]
        let xvfb_child = Some(start_xvfb(opts.window_size).await?);

        args.push("--disable-dev-shm-usage".into());

        let temp_user_data_dir = if let Some(ref dir) = opts.user_data_dir {
            args.push(format!("--user-data-dir={}", dir));
            None
        } else {
            let tmp = PathBuf::from("/tmp").join(format!("chromium-driver-{}", random_hex()));
            args.push(format!("--user-data-dir={}", tmp.display()));
            Some(tmp)
        };

        args.extend(opts.extra_args);

        let mut command = Command::new(&opts.executable);
        command
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .stdin(Stdio::null());

        #[cfg(target_os = "linux")]
        if let Some((_, display)) = xvfb_child.as_ref() {
            command.env("DISPLAY", display);
        }

        let mut child = command.spawn().map_err(CdpError::ProcessStart)?;

        let stderr = child.stderr.take().expect("stderr was piped");
        let mut reader = BufReader::new(stderr).lines();

        let ws_url = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(url) = line.strip_prefix("DevTools listening on ") {
                    return Ok(url.trim().to_owned());
                }
            }
            Err(CdpError::BrowserCrashed)
        })
        .await
        .map_err(|_| CdpError::Timeout(std::time::Duration::from_secs(30)))??;

        Ok(Self {
            child,
            ws_url,
            debug_port: port,
            temp_user_data_dir,
            #[cfg(target_os = "linux")]
            xvfb_child: xvfb_child.map(|(c, _)| c),
            _launch_permit: None,
        })
    }

    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await.map_err(CdpError::ProcessStart)?;
        self.kill_xvfb();
        self.cleanup_temp_dir();
        Ok(())
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.child.wait().await.map_err(CdpError::ProcessStart)?;
        self.kill_xvfb();
        self.cleanup_temp_dir();
        Ok(status)
    }

    fn kill_xvfb(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(xvfb) = self.xvfb_child.as_mut() {
            let _ = xvfb.start_kill();
        }
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
        self.kill_xvfb();
        self.cleanup_temp_dir();
    }
}

/// Starts an Xvfb server and returns the child process plus the `DISPLAY`
/// string (e.g. `":42"`). Uses `-displayfd 1` to let Xvfb pick a free
/// display number and print it on stdout.
#[cfg(target_os = "linux")]
async fn start_xvfb(window_size: (u32, u32)) -> Result<(Child, String)> {
    let screen = format!("{}x{}x24", window_size.0, window_size.1);
    let mut child = Command::new("Xvfb")
        .arg("-displayfd")
        .arg("1")
        .arg("-screen")
        .arg("0")
        .arg(&screen)
        .arg("-nolisten")
        .arg("tcp")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(CdpError::ProcessStart)?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let mut reader = BufReader::new(stdout).lines();

    let timeout = std::time::Duration::from_secs(10);
    let display_num = match tokio::time::timeout(timeout, reader.next_line()).await {
        Ok(Ok(Some(line))) => line.trim().to_owned(),
        _ => {
            let _ = child.start_kill();
            return Err(CdpError::BrowserCrashed);
        }
    };

    if display_num.is_empty() {
        let _ = child.start_kill();
        return Err(CdpError::BrowserCrashed);
    }

    tracing::debug!(display = %display_num, "Xvfb started");
    Ok((child, format!(":{display_num}")))
}
