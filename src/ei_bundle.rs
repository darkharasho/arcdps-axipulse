//! Manage the bundled EI CLI + .NET 8 runtime archives: extract on
//! first run, skip when the on-disk version matches the bundled.
//!
//! EI is framework-dependent and needs hostfxr/.NETCore.App 8 on disk,
//! which Wine prefixes don't ship — bundling both keeps the plugin
//! self-contained.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Version of EI bundled into this build. Bump when re-running
/// `scripts/fetch_ei.sh`.
pub const BUNDLED_EI_VERSION: &str = "v3.22.0.0";

/// Bytes of the bundled GW2EICLI.zip.
pub const BUNDLED_EI_ZIP: &[u8] = include_bytes!("../vendor/GW2EICLI.zip");

/// Version of the .NET runtime bundled. Bump when re-running
/// `scripts/fetch_dotnet.sh`.
pub const BUNDLED_DOTNET_VERSION: &str = "8.0.27";

/// Bytes of the bundled dotnet-runtime-<ver>-win-x64.zip.
pub const BUNDLED_DOTNET_ZIP: &[u8] = include_bytes!("../vendor/dotnet-runtime-win-x64.zip");

pub fn extract_zip(zip_path: &Path, out_dir: &Path) -> io::Result<()> {
    let bytes = fs::read(zip_path)?;
    extract_bytes(&bytes, out_dir)
}

fn extract_bytes(bytes: &[u8], out_dir: &Path) -> io::Result<()> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)?;
    }
    fs::create_dir_all(out_dir)?;

    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let dest = out_dir.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() { fs::create_dir_all(parent)?; }
        let mut out = fs::File::create(&dest)?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        out.write_all(&buf)?;
    }
    Ok(())
}

/// Generic version-gated install: extract `zip_bytes` into
/// `install_root/<subdir>` if `install_root/<marker_name>` doesn't
/// already pin the same `version`.
pub fn install(
    zip_bytes: &[u8],
    version: &str,
    install_root: &Path,
    subdir: &str,
    marker_name: &str,
) -> io::Result<()> {
    let marker = install_root.join(marker_name);
    if let Ok(existing) = fs::read_to_string(&marker) {
        if existing == version {
            return Ok(());
        }
    }
    fs::create_dir_all(install_root)?;
    extract_bytes(zip_bytes, &install_root.join(subdir))?;
    fs::write(&marker, version)?;
    Ok(())
}

/// EI-specific wrapper, kept for the unit test and historical callers.
pub fn install_from_bytes(zip_bytes: &[u8], version: &str, install_root: &Path) -> io::Result<()> {
    install(zip_bytes, version, install_root, "eicli", "eicli-version.txt")
}

/// Install the bundled .NET 8 runtime under `install_root/dotnet`.
pub fn install_dotnet(install_root: &Path) -> io::Result<()> {
    install(
        BUNDLED_DOTNET_ZIP,
        BUNDLED_DOTNET_VERSION,
        install_root,
        "dotnet",
        "dotnet-version.txt",
    )
}

pub fn default_install_root() -> Option<PathBuf> {
    #[cfg(windows)] {
        std::env::var_os("LOCALAPPDATA").map(|s| PathBuf::from(s).join("Axipulse"))
    }
    #[cfg(not(windows))] {
        std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".local/share/axipulse"))
    }
}

pub fn ei_cli_exe(install_root: &Path) -> PathBuf {
    install_root.join("eicli").join("GuildWars2EliteInsights-CLI.exe")
}

/// Directory the EI subprocess should treat as `DOTNET_ROOT`. EI's
/// apphost looks here for `host/fxr/<ver>/hostfxr.dll` and the shared
/// `Microsoft.NETCore.App` framework.
pub fn dotnet_root(install_root: &Path) -> PathBuf {
    install_root.join("dotnet")
}
