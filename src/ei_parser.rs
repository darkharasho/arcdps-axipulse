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

/// GC heap hard limit for the first (capped) EI attempt, hex bytes.
/// Typical WvW fights peak well under 1 GiB total; the biggest log on
/// record needed ~2 GiB uncapped and gets there via the one-shot
/// uncapped retry in `parse_log`. 0x60000000 = 1.5 GiB.
const EI_HEAP_CAP: &str = "0x60000000";

use crate::ei_bundle::{dotnet_root, ei_cli_exe};
use crate::ei_model::EiJson;
use crate::ei_settings::{generate_ei_conf, EiSettings};

#[derive(Debug)]
pub enum ParseError {
    SettingsWrite(std::io::Error),
    SubprocessSpawn(std::io::Error),
    SubprocessExit { code: Option<i32>, stderr: String },
    /// EI exited "successfully" but produced no .json.gz. EI catches
    /// its own parse exceptions and exits 0, so the interesting detail
    /// is whatever it printed — carried here for the log.
    NoJsonOutput { ei_output: String },
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
            Self::NoJsonOutput { ei_output } =>
                write!(f, "EI produced no .json.gz output; EI said: {ei_output}"),
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

    // First attempt runs with the GC heap capped at EI_HEAP_CAP — the
    // overwhelming majority of logs fit, and staying bounded is what
    // keeps the parse from shoving the game into swap on a tight box.
    // EI swallows its own OutOfMemoryException, prints "Parsing
    // Failure … OutOfMemoryException" and exits 0 with *no* output
    // file, so a log that genuinely needs more than the cap surfaces
    // as no-gz + OOM text. Retry those once uncapped (~2 GiB observed
    // on the largest log to date): rare, and strictly better than
    // losing the fight.
    let mut output = run_ei(&exe, &dotnet, &conf_path, log_path, true)?;
    if find_json_gz(&work.0).is_none() && mentions_oom(&output) {
        log::warn!(
            "axipulse: EI hit the {EI_HEAP_CAP}-byte GC heap cap on {log_path:?}; retrying uncapped"
        );
        output = run_ei(&exe, &dotnet, &conf_path, log_path, false)?;
    }
    if !output.status.success() {
        return Err(ParseError::SubprocessExit {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let json_gz = find_json_gz(&work.0)
        .ok_or_else(|| ParseError::NoJsonOutput { ei_output: output_tail(&output) })?;

    let bytes = fs::read(&json_gz).map_err(ParseError::ReadOutput)?;
    // Size the output buffer from the gzip ISIZE trailer instead of a
    // ratio guess. EI JSON compresses ~16x, so the old `len * 4` guess
    // forced read_to_end into doubling reallocs — for a 129 MB payload
    // that meant a ~250 MB final buffer plus ~200 MB of memcpy churn,
    // all inside the game process on a memory-tight box. Exact sizing
    // makes the peak equal the payload and eliminates the reallocs
    // (std's read_to_end probes EOF on a 32-byte stack buffer, so an
    // exactly-sized Vec never grows).
    let cap = gzip_isize(&bytes)
        .filter(|&n| n <= 1_500_000_000)
        .unwrap_or(bytes.len().saturating_mul(4));
    let mut decompressed = Vec::with_capacity(cap);
    {
        let mut gz = flate2::read::GzDecoder::new(&bytes[..]);
        gz.read_to_end(&mut decompressed).map_err(ParseError::Gunzip)?;
    }
    // The compressed copy is dead weight during deserialisation.
    drop(bytes);

    serde_json::from_slice(&decompressed).map_err(ParseError::Deserialise)
}

/// Uncompressed size a single-member gzip stream claims in its ISIZE
/// trailer (last 4 bytes, little-endian, size mod 2³²). `None` when the
/// buffer is too short to be gzip or the trailer reads zero. EI writes
/// single-member streams well under 4 GiB, so this is exact for us;
/// callers must still treat it as a hint and clamp against absurd
/// values from a corrupt trailer.
pub fn gzip_isize(gz: &[u8]) -> Option<usize> {
    if gz.len() < 18 {
        return None;
    }
    let t = &gz[gz.len() - 4..];
    let n = u32::from_le_bytes([t[0], t[1], t[2], t[3]]) as usize;
    (n > 0).then_some(n)
}

/// Spawn the EI CLI on `log_path` and wait for it (10-minute cap).
/// `cap_heap` gates the GC hard limit for the capped-then-retry flow
/// in `parse_log`.
fn run_ei(
    exe: &Path,
    dotnet: &Path,
    conf_path: &Path,
    log_path: &Path,
    cap_heap: bool,
) -> Result<std::process::Output, ParseError> {
    let mut cmd = Command::new(exe);
    cmd.arg("-c").arg(conf_path)
        .arg(log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Point EI's apphost at our bundled .NET 8 instead of relying on the
    // Wine prefix to have a system runtime installed.
    if dotnet.join("dotnet.exe").exists() {
        cmd.env("DOTNET_ROOT", dotnet);
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
    // Bound EI's memory, not just its CPU. Live monitoring showed the
    // post-fight lag was a system-wide zram swap storm: the box runs
    // <1 GB free while the game plays, and the parse-time demand burst
    // tipped the kernel into swapping the game's own pages out (severe
    // multi-second stall). Conserve-memory stays on for both attempts;
    // the hard limit only on the first (see parse_log).
    if cap_heap {
        cmd.env("DOTNET_GCHeapHardLimit", EI_HEAP_CAP);
    }
    // Trade GC CPU for a smaller resident heap (0-9, higher = more
    // aggressive). The subprocess is affinity-confined anyway.
    cmd.env("DOTNET_GCConserveMemory", "7");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW | IDLE_PRIORITY_CLASS);
    let mut child = cmd.spawn().map_err(ParseError::SubprocessSpawn)?;
    #[cfg(windows)]
    confine_child_to_cores(&child, EI_CORES);

    match wait_with_timeout(&mut child, Duration::from_secs(600)) {
        Some(o) => Ok(o),
        None => {
            let _ = child.kill();
            Err(ParseError::SubprocessExit {
                code: None,
                stderr: "EI parse timed out after 10 minutes".to_string(),
            })
        }
    }
}

fn find_json_gz(work_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(work_dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("gz"))
}

/// EI reported an OutOfMemoryException on either stream. Full scan —
/// the streams are a few KB at most.
fn mentions_oom(output: &std::process::Output) -> bool {
    let has = |b: &[u8]| String::from_utf8_lossy(b).to_ascii_lowercase().contains("outofmemory");
    has(&output.stdout) || has(&output.stderr)
}

/// Combined stdout+stderr, clipped to the last ~400 chars — enough to
/// carry EI's "Parsing Failure - …: <reason>" line into arcdps.log.
fn output_tail(output: &std::process::Output) -> String {
    let mut s = String::from_utf8_lossy(&output.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&output.stderr));
    let s = s.trim();
    let mut start = s.len().saturating_sub(400);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    s[start..].to_string()
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
