//! Automatic updates from GitHub releases.
//!
//! The application checks the latest GitHub release on startup.  When a newer
//! version is available the user can download it and restart into the new
//! build.  The running executable cannot replace itself on Windows, so a small
//! PowerShell helper waits for the process to exit, copies the new executable
//! over the old one and starts it again.

use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

const REPO: &str = "Latte-Night-Studio/RustProjector";
const USER_AGENT: &str = concat!("RustProjector/", env!("CARGO_PKG_VERSION"));
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Information about a release newer than the running application.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub page_url: String,
    pub download_url: String,
    pub sha256_url: Option<String>,
}

enum UpdateEvent {
    Checked(Result<Option<UpdateInfo>, String>),
    Downloaded(Result<PathBuf, String>),
}

/// Background update state shared by the settings page and the update banner.
#[derive(Default)]
pub struct UpdateState {
    pub info: Option<UpdateInfo>,
    pub error: Option<String>,
    pub checking: bool,
    pub downloading: bool,
    pub ready: Option<PathBuf>,
    /// When the download finished.  The install button stays disabled for a
    /// moment so that the mouse release which started the download cannot
    /// also trigger the installation.
    pub ready_at: Option<Instant>,
    /// True after at least one check has finished (successfully or not).
    pub checked: bool,
    /// True when the user hid the update banner for this version.
    pub dismissed: bool,
    /// True when the running check was started automatically at startup.
    silent: bool,
    receiver: Option<Receiver<UpdateEvent>>,
}

impl UpdateState {
    /// Start the automatic startup check.  Does nothing if a check is running.
    pub fn check_now(&mut self) {
        self.start_check(false);
    }

    /// Check at startup without showing errors when the computer is offline.
    pub fn check_at_startup(&mut self) {
        self.start_check(true);
    }

    fn start_check(&mut self, silent: bool) {
        if self.checking || self.downloading {
            return;
        }
        self.checking = true;
        self.error = None;
        self.dismissed = false;
        self.silent = silent;
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(UpdateEvent::Checked(check_for_update()));
        });
    }

    /// Download the release archive and verify its checksum.
    pub fn download(&mut self) {
        let Some(info) = self.info.clone() else {
            return;
        };
        if self.checking || self.downloading {
            return;
        }
        self.downloading = true;
        self.error = None;
        self.ready = None;
        self.ready_at = None;
        self.dismissed = false;
        self.silent = false;
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(UpdateEvent::Downloaded(download_and_stage(&info)));
        });
    }

    /// Handle messages from the background worker.
    pub fn poll(&mut self) {
        let Some(receiver) = self.receiver.as_ref() else {
            return;
        };
        let message = match receiver.try_recv() {
            Ok(message) => Some(message),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.checking = false;
                self.downloading = false;
                if !self.silent {
                    self.error = Some("Фонове завдання оновлення несподівано завершилося.".into());
                }
                return;
            }
        };
        let Some(message) = message else {
            return;
        };
        self.receiver = None;
        match message {
            UpdateEvent::Checked(result) => {
                self.checking = false;
                self.checked = true;
                match result {
                    Ok(info) => {
                        self.info = info;
                        self.error = None;
                    }
                    Err(error) => {
                        self.info = None;
                        if !self.silent {
                            self.error = Some(error);
                        }
                    }
                }
            }
            UpdateEvent::Downloaded(result) => {
                self.downloading = false;
                match result {
                    Ok(path) => {
                        self.ready = Some(path);
                        self.ready_at = Some(Instant::now());
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
    }

    /// True once a staged update has been available long enough for a
    /// deliberate click.  This prevents the release of the click that started
    /// the download from also activating the install button that appears in
    /// the same place.
    pub fn can_install(&self) -> bool {
        self.ready.is_some()
            && self
                .ready_at
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(600))
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent(USER_AGENT)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

fn http_text(url: &str) -> Result<String, String> {
    let response = agent()
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("Не вдалося зв'язатися з GitHub: {error}"))?;
    response
        .into_string()
        .map_err(|error| format!("Не вдалося прочитати відповідь GitHub: {error}"))
}

fn http_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = agent()
        .get(url)
        .call()
        .map_err(|error| format!("Не вдалося завантажити файл: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Не вдалося прочитати файл: {error}"))?;
    Ok(bytes)
}

/// Query the latest GitHub release and return it when it is newer than us.
fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = http_text(&url)?;
    parse_release(&body, CURRENT_VERSION)
}

/// Parse a GitHub release response and select the Windows update archive.
fn parse_release(body: &str, current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let release: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("Некоректна відповідь GitHub: {error}"))?;

    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| "У відповіді GitHub немає тегу релізу.".to_string())?
        .to_string();
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    if !is_newer(&version, current_version) {
        return Ok(None);
    }

    let page_url = release["html_url"].as_str().unwrap_or_default().to_string();
    let notes = release["body"].as_str().unwrap_or_default().to_string();
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| "У відповіді GitHub немає списку файлів.".to_string())?;

    let archive = assets
        .iter()
        .find(|asset| {
            let name = asset["name"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            name.ends_with(".zip") && (name.contains("windows") || name.contains("x64"))
        })
        .or_else(|| {
            assets.iter().find(|asset| {
                asset["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .ends_with(".zip")
            })
        })
        .ok_or_else(|| "У релізі немає архіву оновлення для Windows.".to_string())?;

    let asset_name = archive["name"].as_str().unwrap_or_default().to_string();
    let download_url = archive["browser_download_url"]
        .as_str()
        .ok_or_else(|| "Не вдалося отримати посилання на архів оновлення.".to_string())?
        .to_string();
    let sha256_name = format!("{asset_name}.sha256");
    let sha256_url = assets.iter().find_map(|asset| {
        (asset["name"].as_str() == Some(sha256_name.as_str()))
            .then(|| asset["browser_download_url"].as_str().map(str::to_string))
            .flatten()
    });

    Ok(Some(UpdateInfo {
        version,
        notes,
        page_url,
        download_url,
        sha256_url,
    }))
}

fn download_and_stage(info: &UpdateInfo) -> Result<PathBuf, String> {
    let bytes = http_bytes(&info.download_url)?;

    if let Some(checksum_url) = &info.sha256_url {
        let checksum_text = http_text(checksum_url)?;
        let expected = checksum_text
            .split_whitespace()
            .next()
            .ok_or_else(|| "Файл контрольної суми порожній.".to_string())?;
        let actual = hex_sha256(&bytes);
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(format!(
                "Контрольна сума оновлення не збігається (очікувано {expected}, отримано {actual})."
            ));
        }
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("Не вдалося відкрити архів оновлення: {error}"))?;
    let mut executable = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Не вдалося прочитати архів оновлення: {error}"))?;
        let name = entry.name().to_string();
        let normalized = name.replace('\\', "/").to_ascii_lowercase();
        let is_executable =
            normalized == "softprojector.exe" || normalized.ends_with("/softprojector.exe");
        if is_executable {
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|error| format!("Не вдалося розпакувати програму: {error}"))?;
            executable = Some(data);
            break;
        }
    }
    let executable =
        executable.ok_or_else(|| "В архіві оновлення немає softprojector.exe.".to_string())?;

    let staging = std::env::temp_dir()
        .join("RustProjector")
        .join(format!("update-{}", info.version));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|error| format!("Не вдалося створити теку оновлення: {error}"))?;
    let staged_exe = staging.join("softprojector.exe");
    std::fs::write(&staged_exe, executable)
        .map_err(|error| format!("Не вдалося зберегти оновлення: {error}"))?;
    Ok(staged_exe)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// Compare numeric version components: `0.10.0` is newer than `0.9.9`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let candidate = version_parts(candidate);
    let current = version_parts(current);
    let length = candidate.len().max(current.len());
    for index in 0..length {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = current.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .trim_start_matches(['v', 'V'])
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

/// Start the helper that replaces the executable after this process exits.
#[cfg(windows)]
pub fn launch_installer(staged: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let target = std::env::current_exe()
        .map_err(|error| format!("Не вдалося визначити шлях програми: {error}"))?;
    let script_path = staged
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("install-update.ps1");
    std::fs::write(&script_path, UPDATE_SCRIPT)
        .map_err(|error| format!("Не вдалося створити сценарій оновлення: {error}"))?;

    let mut command = std::process::Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script_path)
        .arg("-ProcessId")
        .arg(std::process::id().to_string())
        .arg("-Source")
        .arg(staged)
        .arg("-Target")
        .arg(&target)
        .creation_flags(CREATE_NO_WINDOW);
    command
        .spawn()
        .map_err(|error| format!("Не вдалося запустити оновлення: {error}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn launch_installer(_staged: &Path) -> Result<(), String> {
    Err("Автоматичне оновлення підтримується лише у Windows.".into())
}

#[cfg(windows)]
const UPDATE_SCRIPT: &str = r#"param(
    [Parameter(Mandatory=$true)][int]$ProcessId,
    [Parameter(Mandatory=$true)][string]$Source,
    [Parameter(Mandatory=$true)][string]$Target
)

$ErrorActionPreference = 'SilentlyContinue'
$deadline = (Get-Date).AddSeconds(90)
while ((Get-Process -Id $ProcessId -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {
    Start-Sleep -Milliseconds 400
}
Start-Sleep -Milliseconds 800

$copied = $false
for ($attempt = 0; $attempt -lt 20 -and -not $copied; $attempt++) {
    try {
        Copy-Item -LiteralPath $Source -Destination $Target -Force -ErrorAction Stop
        $copied = $true
    } catch {
        Start-Sleep -Milliseconds 500
    }
}

if (-not $copied) {
    # The installation directory may require administrator rights.
    try {
        Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList @(
            '-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden',
            '-File', $PSCommandPath,
            '-ProcessId', $ProcessId,
            '-Source', $Source,
            '-Target', $Target
        )
    } catch {
    }
    exit 1
}

$directory = Split-Path -Parent $Target
Start-Process -FilePath $Target -WorkingDirectory $directory
Start-Sleep -Seconds 1
Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn formats_sha256() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn debounces_install_button() {
        let mut state = UpdateState::default();
        assert!(!state.can_install());

        state.ready = Some(PathBuf::from("softprojector.exe"));
        assert!(!state.can_install());

        state.ready_at = Some(Instant::now() - Duration::from_secs(1));
        assert!(state.can_install());
    }

    #[test]
    fn parses_newer_release() {
        let body = r#"{
            "tag_name": "v0.3.0",
            "html_url": "https://example.test/release",
            "body": "Нотатки до випуску",
            "assets": [
                {
                    "name": "RustProjector-v0.3.0-windows-x64.zip",
                    "browser_download_url": "https://example.test/app.zip"
                },
                {
                    "name": "RustProjector-v0.3.0-windows-x64.zip.sha256",
                    "browser_download_url": "https://example.test/app.zip.sha256"
                }
            ]
        }"#;
        let info = parse_release(body, "0.2.0").unwrap().unwrap();
        assert_eq!(info.version, "0.3.0");
        assert_eq!(info.notes, "Нотатки до випуску");
        assert_eq!(info.page_url, "https://example.test/release");
        assert_eq!(info.download_url, "https://example.test/app.zip");
        assert_eq!(
            info.sha256_url.as_deref(),
            Some("https://example.test/app.zip.sha256")
        );
        assert!(parse_release(body, "0.3.0").unwrap().is_none());
        assert!(parse_release(body, "0.3.1").unwrap().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn update_script_replaces_target() {
        let directory = std::env::temp_dir().join(format!(
            "rustprojector-update-script-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.exe");
        let target = directory.join("target.exe");
        let script = directory.join("install-update.ps1");

        let system_exe = std::path::Path::new(r"C:\Windows\System32\where.exe");
        if !system_exe.exists() {
            return;
        }
        let mut new_bytes = std::fs::read(system_exe).unwrap();
        new_bytes.extend_from_slice(b"rustprojector-update-test-marker");
        std::fs::write(&source, &new_bytes).unwrap();
        std::fs::write(&target, std::fs::read(system_exe).unwrap()).unwrap();
        std::fs::write(&script, UPDATE_SCRIPT).unwrap();

        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-File",
            ])
            .arg(&script)
            .arg("-ProcessId")
            .arg("999999")
            .arg("-Source")
            .arg(&source)
            .arg("-Target")
            .arg(&target)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read(&target).unwrap(), new_bytes);
        assert!(!script.exists());
        std::thread::sleep(Duration::from_millis(500));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn downloads_verifies_and_stages_update() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use zip::write::SimpleFileOptions;

        let mut archive_bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut archive_bytes));
            writer
                .start_file("softprojector.exe", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"new executable").unwrap();
            writer.finish().unwrap();
        }
        let checksum = format!("{}  update.zip\n", hex_sha256(&archive_bytes));

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let read = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..read]);
                let (status, body) = if request.starts_with("GET /update.zip.sha256") {
                    ("200 OK", checksum.as_bytes().to_vec())
                } else if request.starts_with("GET /update.zip") {
                    ("200 OK", archive_bytes.clone())
                } else {
                    ("404 Not Found", b"not found".to_vec())
                };
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(header.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });

        let info = UpdateInfo {
            version: "9.9.9-test".into(),
            notes: String::new(),
            page_url: String::new(),
            download_url: format!("http://{address}/update.zip"),
            sha256_url: Some(format!("http://{address}/update.zip.sha256")),
        };
        let staged = download_and_stage(&info).unwrap();
        assert_eq!(std::fs::read(&staged).unwrap(), b"new executable");
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(staged.parent().unwrap());
    }
}
