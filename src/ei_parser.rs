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

/// Win32 `IDLE_PRIORITY_CLASS` — EI only runs when GW2 has nothing
/// else it wants the CPU for. BELOW_NORMAL still let the .NET 8 apphost
/// cold-start burst cause a ~1s render-thread stutter under Wine at
/// parse-start; IDLE makes the subprocess fully yield instead.
#[cfg(windows)]
const IDLE_PRIORITY_CLASS: u32 = 0x0000_0040;

/// How many logical CPUs the EI subprocess may use. Kept small so GW2's
/// render/worker threads always have uncontended cores during a parse;
/// EI takes a little longer but the game never hitches.
const EI_CORES: u32 = 2;

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
    // Tame the .NET cold-start burst that freezes GW2's render thread under
    // Wine. IDLE_PRIORITY_CLASS only throttles CPU *scheduling*; it does
    // nothing about the CLR spawning a GC heap + dedicated GC thread per
    // logical CPU at startup. Under server GC on a many-core box that's a
    // dozen-plus thread creations crammed into the first few ms, each one
    // serialized through Wine's single-threaded wineserver — which is what
    // stalls the render thread. Force workstation GC with a single heap so
    // startup creates one GC thread instead of N, and disable background
    // (concurrent) GC so there's no extra background collector thread either.
    cmd.env("DOTNET_gcServer", "0");
    cmd.env("DOTNET_GCHeapCount", "1");
    cmd.env("DOTNET_gcConcurrent", "0");
    // IDLE_PRIORITY_CLASS is a no-op under default Wine (priority classes
    // don't map to Unix nice without extra privileges), so EI still
    // competes with GW2's render thread on every core. Affinity *is*
    // honoured (sched_setaffinity), so confine EI to EI_CORES cores via
    // SetProcessAffinityMask after spawn, and tell the runtime the same
    // number so the thread pool / Parallel.For / JIT sizing all shrink
    // to match instead of spawning a per-logical-CPU thread burst
    // through the single-threaded wineserver.
    cmd.env("DOTNET_PROCESSOR_COUNT", EI_CORES.to_string());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW | IDLE_PRIORITY_CLASS);
    let mut child = cmd.spawn().map_err(ParseError::SubprocessSpawn)?;
    #[cfg(windows)]
    confine_child_to_cores(&child, EI_CORES);

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

/// Pin `child` to the `n` highest available logical CPUs. Highest, not
/// lowest, to stay away from core 0 where Wine parks interrupt-heavy
/// work. Process affinity applies to threads the child has already
/// created, so calling right after spawn covers the .NET startup burst.
/// Best-effort: on failure EI just runs unconfined, as before.
#[cfg(windows)]
fn confine_child_to_cores(child: &std::process::Child, n: u32) {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetProcessAffinityMask, SetProcessAffinityMask,
    };
    unsafe {
        let mut proc_mask: usize = 0;
        let mut sys_mask: usize = 0;
        if GetProcessAffinityMask(GetCurrentProcess(), &mut proc_mask, &mut sys_mask).is_err() {
            return;
        }
        let mut mask: usize = 0;
        let mut left = n;
        for bit in (0..usize::BITS).rev() {
            if left == 0 { break; }
            if sys_mask & (1usize << bit) != 0 {
                mask |= 1usize << bit;
                left -= 1;
            }
        }
        if mask == 0 { return; }
        let _ = SetProcessAffinityMask(HANDLE(child.as_raw_handle()), mask);
    }
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
