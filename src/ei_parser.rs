//! Run the bundled EI CLI against an .evtc/.zevtc and return the parsed JSON.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Win32 `CREATE_NO_WINDOW` — suppresses the console window that Windows
/// would otherwise allocate for a CUI subprocess spawned from a GUI app
/// like GW2. Without this flag EI flashes a black terminal on every parse.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::ei_bundle::{dotnet_root, ei_cli_exe};
use crate::ei_model::EiJson;
use crate::ei_settings::{generate_ei_conf, EiSettings};

#[derive(Debug)]
pub enum ParseError {
    SettingsWrite(std::io::Error),
    SubprocessSpawn(std::io::Error),
    SubprocessExit { code: Option<i32>, stderr: String },
    NoJsonOutput,
    ReadOutput(std::io::Error),
    Gunzip(std::io::Error),
    Deserialise(serde_json::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SettingsWrite(e)   => write!(f, "writing settings.conf: {e}"),
            Self::SubprocessSpawn(e) => write!(f, "spawning EI CLI: {e}"),
            Self::SubprocessExit { code, stderr } =>
                write!(f, "EI CLI exited code={code:?}; stderr={stderr}"),
            Self::NoJsonOutput       => write!(f, "EI produced no .json.gz output"),
            Self::ReadOutput(e)      => write!(f, "reading EI JSON output: {e}"),
            Self::Gunzip(e)          => write!(f, "gunzip EI output: {e}"),
            Self::Deserialise(e)     => write!(f, "deserialising EI JSON: {e}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// RAII guard: removes the temp dir on drop so every error path cleans up.
struct WorkDir(PathBuf);

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn parse_log(
    install_root: &Path,
    settings: &EiSettings,
    log_path: &Path,
) -> Result<EiJson, ParseError> {
    let work = WorkDir(mktempdir(install_root).map_err(ParseError::SettingsWrite)?);
    let conf_path = work.0.join("settings.conf");
    fs::write(&conf_path, generate_ei_conf(settings, work.0.to_string_lossy().as_ref()))
        .map_err(ParseError::SettingsWrite)?;

    let exe = ei_cli_exe(install_root);
    let dotnet = dotnet_root(install_root);
    let mut cmd = Command::new(&exe);
    cmd.arg("-c").arg(&conf_path)
        .arg(log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Point EI's apphost at our bundled .NET 8 instead of relying on the
    // Wine prefix to have a system runtime installed.
    if dotnet.join("dotnet.exe").exists() {
        cmd.env("DOTNET_ROOT", &dotnet);
    }
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn().map_err(ParseError::SubprocessSpawn)?;

    let timeout = Duration::from_secs(600);
    let output = match wait_with_timeout(&mut child, timeout) {
        Some(o) => o,
        None => {
            let _ = child.kill();
            return Err(ParseError::SubprocessExit {
                code: None,
                stderr: "EI parse timed out after 10 minutes".to_string(),
            });
        }
    };
    if !output.status.success() {
        return Err(ParseError::SubprocessExit {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let json_gz = fs::read_dir(&work.0)
        .map_err(ParseError::ReadOutput)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("gz"))
        .ok_or(ParseError::NoJsonOutput)?;

    let bytes = fs::read(&json_gz).map_err(ParseError::ReadOutput)?;
    let mut gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut decompressed = Vec::with_capacity(bytes.len() * 4);
    gz.read_to_end(&mut decompressed).map_err(ParseError::Gunzip)?;

    serde_json::from_slice(&decompressed).map_err(ParseError::Deserialise)
}

fn mktempdir(root: &Path) -> std::io::Result<PathBuf> {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos()).unwrap_or(0);
    let dir = root.join(format!("ei-parse-{pid}-{nanos}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> Option<std::process::Output> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait().ok().flatten() {
            Some(_status) => {
                let stdout = child.stdout.take().map(read_all).unwrap_or_default();
                let stderr = child.stderr.take().map(read_all).unwrap_or_default();
                let status = child.wait().ok()?;
                return Some(std::process::Output { status, stdout, stderr });
            }
            None => {
                if start.elapsed() >= timeout { return None; }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn read_all<R: std::io::Read>(mut r: R) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = r.read_to_end(&mut out);
    out
}
