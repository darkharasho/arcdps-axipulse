#![cfg(windows)]
//! Sidecar JPEG tile texture cache. On first lookup of a (zoom, tx, ty)
//! tuple we load the JPEG from `<install_root>/axipulse-assets/tiles/
//! {z}/{tx}/{ty}.jpg`, decode to RGBA8, upload to a D3D11 texture, and
//! cache the SRV pointer for ImGui to consume as a `TextureId`.
//!
//! Mirrors the lifetime model of `crate::ui::icons`: textures live for
//! the lifetime of the plugin. Frame-drain to avoid CreateTexture2D
//! bursts: at most `MAX_UPLOADS_PER_FRAME` uploads per `drain_pending`
//! call.

use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

use arcdps::imgui::TextureId;
use once_cell::sync::Lazy;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE,
    D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

const MAX_UPLOADS_PER_FRAME: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey { pub zoom: u32, pub tx: u32, pub ty: u32 }

#[derive(Clone, Copy)]
pub struct TileHandle { pub tex: TextureId }

enum TileState {
    Pending(Vec<u8>), // raw JPEG bytes loaded from disk, awaiting upload
    Ready { ptr: usize },
    Failed,
}

static TILES: Lazy<Mutex<HashMap<TileKey, TileState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
// Hold SRVs alive for plugin lifetime (TextureId is just the raw ptr).
static SRVS: Lazy<Mutex<Vec<ID3D11ShaderResourceView>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

fn assets_root() -> Option<PathBuf> {
    crate::plugin::install_root()
        .map(|p| {
            let mut buf = p;
            // If install_root points at the DLL file itself, parent it.
            if buf.is_file() {
                if let Some(parent) = buf.parent() { buf = parent.to_path_buf(); }
            }
            buf.push("axipulse-assets");
            buf.push("tiles");
            buf
        })
        .filter(|p| p.exists())
}

/// Look up a tile. If not yet seen, schedule a disk load + queue upload.
/// Returns `None` until the texture is uploaded.
pub fn lookup(key: TileKey) -> Option<TileHandle> {
    let mut guard = TILES.lock().ok()?;
    match guard.get(&key) {
        Some(TileState::Ready { ptr }) => Some(TileHandle { tex: TextureId::new(*ptr) }),
        Some(TileState::Failed) => None,
        Some(TileState::Pending(_)) => None,
        None => {
            // Load JPEG bytes from disk (cheap; bounded by tile size ~20KB).
            let Some(root) = assets_root() else {
                guard.insert(key, TileState::Failed);
                return None;
            };
            let path = root.join(format!("{}/{}/{}.jpg", key.zoom, key.tx, key.ty));
            match std::fs::read(&path) {
                Ok(bytes) => { guard.insert(key, TileState::Pending(bytes)); }
                Err(_) => { guard.insert(key, TileState::Failed); }
            }
            None
        }
    }
}

/// Call once per imgui frame. Uploads up to `MAX_UPLOADS_PER_FRAME`
/// pending tiles to D3D11. Safe to call when no device is available
/// (no-op until arcdps has handed us one).
pub fn drain_pending() {
    let device = match arcdps::d3d11_device() {
        Some(d) => d,
        None => return,
    };
    let mut to_upload: Vec<(TileKey, Vec<u8>)> = Vec::new();
    {
        let mut guard = match TILES.lock() { Ok(g) => g, Err(_) => return };
        for (key, state) in guard.iter_mut() {
            if to_upload.len() >= MAX_UPLOADS_PER_FRAME { break; }
            if let TileState::Pending(_) = state {
                let TileState::Pending(bytes) = std::mem::replace(state, TileState::Failed) else { unreachable!() };
                to_upload.push((*key, bytes));
            }
        }
    }

    for (key, bytes) in to_upload {
        let result = decode_and_upload(&device, &bytes);
        let mut guard = match TILES.lock() { Ok(g) => g, Err(_) => continue };
        match result {
            Some(ptr) => { guard.insert(key, TileState::Ready { ptr }); }
            None      => { guard.insert(key, TileState::Failed); }
        }
    }
}

fn decode_and_upload(device: &ID3D11Device, jpeg_bytes: &[u8]) -> Option<usize> {
    match unsafe { upload_rgba(device, jpeg_bytes) } {
        Ok(ptr) => Some(ptr),
        Err(e) => {
            log::warn!("axipulse tile_cache: upload failed: {e}");
            None
        }
    }
}

unsafe fn upload_rgba(
    device: &ID3D11Device,
    bytes: &[u8],
) -> Result<usize, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    let pixels = img.into_raw();
    let desc = D3D11_TEXTURE2D_DESC {
        Width: w,
        Height: h,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        ..Default::default()
    };
    let init_data = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr() as *const c_void,
        SysMemPitch: w * 4,
        SysMemSlicePitch: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    device.CreateTexture2D(&desc, Some(&init_data), Some(&mut tex))?;
    let tex = tex.ok_or("CreateTexture2D returned null")?;
    let mut srv: Option<ID3D11ShaderResourceView> = None;
    device.CreateShaderResourceView(&tex, None, Some(&mut srv))?;
    let srv = srv.ok_or("CreateShaderResourceView returned null")?;
    let ptr = srv.as_raw() as usize;
    if let Ok(mut s) = SRVS.lock() { s.push(srv); }
    Ok(ptr)
}
