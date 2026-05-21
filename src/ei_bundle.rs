//! Manage the bundled EI CLI archive: extract on first run, skip when
//! the on-disk version matches the bundled version.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Version of EI bundled into this build. Bump when re-running
/// `scripts/fetch_ei.sh`.
pub const BUNDLED_EI_VERSION: &str = "0.0.0-replace-on-fetch";

/// Bytes of the bundled GW2EICLI.zip.
pub const BUNDLED_EI_ZIP: &[u8] = include_bytes!("../vendor/GW2EICLI.zip");

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

pub fn install_from_bytes(zip_bytes: &[u8], version: &str, install_root: &Path) -> io::Result<()> {
    let marker = install_root.join("eicli-version.txt");
    if let Ok(existing) = fs::read_to_string(&marker) {
        if existing == version {
            return Ok(());
        }
    }
    fs::create_dir_all(install_root)?;
    extract_bytes(zip_bytes, &install_root.join("eicli"))?;
    fs::write(&marker, version)?;
    Ok(())
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
