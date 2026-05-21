#![cfg(windows)]
//! Skill / buff icon texture cache.
//!
//! EI's `skillMap` and `buffMap` give us a URL per asset
//! (render.guildwars2.com). On first sight of an ID we spawn a
//! background fetch; once the bytes arrive (or hit the disk cache)
//! the main imgui thread uploads them to a D3D11 texture and stores
//! the SRV pointer for ImGui to consume as a `TextureId`. Lookups
//! that are not yet ready return `None` and callers fall back to
//! their text-only layout.

use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

use arcdps::imgui::TextureId;
use once_cell::sync::Lazy;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE,
    D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

use crate::ei_model::EiJson;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconKind { Skill, Buff }

/// Static, stable string keys for assets bundled into the DLL via
/// `include_bytes!`. Class names match GW2 elite-spec / profession
/// display strings exactly (e.g. `"Firebrand"`, `"Guardian"`); the
/// special key `"__logo__"` is the AxiPulse logo PNG.
const BUNDLED_PNGS: &[(&str, &[u8])] = &[
    ("__logo__",     include_bytes!("../assets/axipulse-logo.png")),
    ("__heartbeat__",include_bytes!("../assets/heartbeat.png")),
    ("Amalgam",      include_bytes!("../assets/classes/Amalgam.png")),
    ("Antiquary",    include_bytes!("../assets/classes/Antiquary.png")),
    ("Berserker",    include_bytes!("../assets/classes/Berserker.png")),
    ("Bladesworn",   include_bytes!("../assets/classes/Bladesworn.png")),
    ("Catalyst",     include_bytes!("../assets/classes/Catalyst.png")),
    ("Chronomancer", include_bytes!("../assets/classes/Chronomancer.png")),
    ("Conduit",      include_bytes!("../assets/classes/Conduit.png")),
    ("Daredevil",    include_bytes!("../assets/classes/Daredevil.png")),
    ("Deadeye",      include_bytes!("../assets/classes/Deadeye.png")),
    ("Dragonhunter", include_bytes!("../assets/classes/Dragonhunter.png")),
    ("Druid",        include_bytes!("../assets/classes/Druid.png")),
    ("Elementalist", include_bytes!("../assets/classes/Elementalist.png")),
    ("Engineer",     include_bytes!("../assets/classes/Engineer.png")),
    ("Evoker",       include_bytes!("../assets/classes/Evoker.png")),
    ("Firebrand",    include_bytes!("../assets/classes/Firebrand.png")),
    ("Galeshot",     include_bytes!("../assets/classes/Galeshot.png")),
    ("Guardian",     include_bytes!("../assets/classes/Guardian.png")),
    ("Harbinger",    include_bytes!("../assets/classes/Harbinger.png")),
    ("Herald",       include_bytes!("../assets/classes/Herald.png")),
    ("Holosmith",    include_bytes!("../assets/classes/Holosmith.png")),
    ("Luminary",     include_bytes!("../assets/classes/Luminary.png")),
    ("Mechanist",    include_bytes!("../assets/classes/Mechanist.png")),
    ("Mesmer",       include_bytes!("../assets/classes/Mesmer.png")),
    ("Mirage",       include_bytes!("../assets/classes/Mirage.png")),
    ("Necromancer",  include_bytes!("../assets/classes/Necromancer.png")),
    ("Paragon",      include_bytes!("../assets/classes/Paragon.png")),
    ("Ranger",       include_bytes!("../assets/classes/Ranger.png")),
    ("Reaper",       include_bytes!("../assets/classes/Reaper.png")),
    ("Renegade",     include_bytes!("../assets/classes/Renegade.png")),
    ("Revenant",     include_bytes!("../assets/classes/Revenant.png")),
    ("Ritualist",    include_bytes!("../assets/classes/Ritualist.png")),
    ("Scourge",      include_bytes!("../assets/classes/Scourge.png")),
    ("Scrapper",     include_bytes!("../assets/classes/Scrapper.png")),
    ("Soulbeast",    include_bytes!("../assets/classes/Soulbeast.png")),
    ("Specter",      include_bytes!("../assets/classes/Specter.png")),
    ("Spellbreaker", include_bytes!("../assets/classes/Spellbreaker.png")),
    ("Tempest",      include_bytes!("../assets/classes/Tempest.png")),
    ("Thief",        include_bytes!("../assets/classes/Thief.png")),
    ("Troubadour",   include_bytes!("../assets/classes/Troubadour.png")),
    ("Untamed",      include_bytes!("../assets/classes/Untamed.png")),
    ("Vindicator",   include_bytes!("../assets/classes/Vindicator.png")),
    ("Virtuoso",     include_bytes!("../assets/classes/Virtuoso.png")),
    ("Warrior",      include_bytes!("../assets/classes/Warrior.png")),
    ("Weaver",       include_bytes!("../assets/classes/Weaver.png")),
    ("Willbender",   include_bytes!("../assets/classes/Willbender.png")),
];

static BUNDLED: Lazy<Mutex<HashMap<&'static str, BundledState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static BUNDLED_SRVS: Lazy<Mutex<Vec<ID3D11ShaderResourceView>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

enum BundledState { Failed, Ready { ptr: usize, aspect: f32 } }

/// Look up a bundled asset by its key (`&str` matched against the
/// table). Loads on first call after a D3D11 device becomes
/// available. Returns `None` while the device isn't ready or the key
/// doesn't match a bundled asset.
pub fn lookup_bundled(key: &str) -> Option<IconHandle> {
    let static_key = BUNDLED_PNGS.iter().find(|(k, _)| *k == key).map(|(k, _)| *k)?;
    {
        let map = BUNDLED.lock().ok()?;
        if let Some(state) = map.get(static_key) {
            return match state {
                BundledState::Ready { ptr, aspect } =>
                    Some(IconHandle { tex: TextureId::new(*ptr), aspect: *aspect }),
                _ => None,
            };
        }
    }
    let device = arcdps::d3d11_device()?;
    let bytes = BUNDLED_PNGS.iter().find(|(k, _)| *k == static_key)?.1;
    let new_state = match unsafe { upload(&device, bytes) } {
        Ok((srv, aspect)) => {
            let ptr = srv.as_raw() as usize;
            if let Ok(mut s) = BUNDLED_SRVS.lock() { s.push(srv); }
            BundledState::Ready { ptr, aspect }
        }
        Err(e) => {
            log::warn!("axipulse bundled icon upload failed for {static_key}: {e}");
            BundledState::Failed
        }
    };
    if let Ok(mut m) = BUNDLED.lock() { m.insert(static_key, new_state); }
    lookup_bundled(static_key)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconKey { pub kind: IconKind, pub id: i64 }

#[derive(Clone, Copy)]
pub struct IconHandle { pub tex: TextureId, pub aspect: f32 }

enum State {
    Pending,
    Failed,
    Ready { ptr: usize, aspect: f32 },
}

struct Cache {
    by_key: HashMap<IconKey, State>,
    /// We keep SRVs alive for the lifetime of the process; ImGui holds
    /// raw pointers into these COM objects every frame.
    _srvs: Vec<ID3D11ShaderResourceView>,
}

unsafe impl Send for Cache {}
unsafe impl Sync for Cache {}

static CACHE: Lazy<Mutex<Cache>> = Lazy::new(|| Mutex::new(Cache {
    by_key: HashMap::new(),
    _srvs: Vec::new(),
}));

type DownloadResult = (IconKey, Result<Vec<u8>, String>);
type DownloadRequest = (IconKey, String, Option<PathBuf>);

struct Chan {
    /// Completed downloads, drained on the imgui thread to upload SRVs.
    tx: Sender<DownloadResult>,
    rx: Mutex<Receiver<DownloadResult>>,
    /// Requests queued for the single worker thread.
    req_tx: Sender<DownloadRequest>,
}

/// Upper bound on D3D11 uploads per imgui frame. Big fights can yield
/// 100+ unmet skill IDs at once; cramming them all into one frame has
/// crashed the host under Wine. Spreading them across frames keeps the
/// GPU load steady.
const MAX_UPLOADS_PER_FRAME: usize = 4;

static CHAN: Lazy<Chan> = Lazy::new(|| {
    let (tx, rx) = mpsc::channel::<DownloadResult>();
    let (req_tx, req_rx) = mpsc::channel::<DownloadRequest>();
    // Single dedicated worker thread runs all HTTP fetches in serial.
    // Replaces the previous "thread::spawn per icon" model which fanned
    // out hundreds of threads at once on big fights.
    let result_tx = tx.clone();
    thread::Builder::new()
        .name("axipulse-icon-worker".into())
        .spawn(move || {
            for (key, url, path) in req_rx {
                match ureq::get(&url).timeout(std::time::Duration::from_secs(20)).call() {
                    Ok(resp) => {
                        let mut bytes: Vec<u8> = Vec::new();
                        if let Err(e) = std::io::copy(&mut resp.into_reader(), &mut bytes) {
                            let _ = result_tx.send((key, Err(e.to_string())));
                            continue;
                        }
                        if let Some(p) = path {
                            if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
                            let _ = std::fs::write(&p, &bytes);
                        }
                        let _ = result_tx.send((key, Ok(bytes)));
                    }
                    Err(e) => { let _ = result_tx.send((key, Err(e.to_string()))); }
                }
            }
        })
        .ok();
    Chan { tx, rx: Mutex::new(rx), req_tx }
});

/// Look up an icon by `(kind, id)`. Returns `Some` once the texture has
/// been uploaded; in the meantime kicks off a download/disk-load and
/// returns `None` so the caller can fall back to a placeholder.
pub fn lookup(json: &EiJson, key: IconKey) -> Option<IconHandle> {
    {
        let cache = CACHE.lock().ok()?;
        if let Some(state) = cache.by_key.get(&key) {
            return match state {
                State::Ready { ptr, aspect } =>
                    Some(IconHandle { tex: TextureId::new(*ptr), aspect: *aspect }),
                _ => None,
            };
        }
    }
    // First sighting — resolve URL.
    let url = match key.kind {
        IconKind::Skill => json.skill_map.get(&format!("s{}", key.id))
            .and_then(|e| e.icon.clone()),
        IconKind::Buff  => json.buff_map.get(&format!("b{}", key.id))
            .and_then(|e| e.icon.clone()),
    };
    let url = match url {
        Some(u) if !u.is_empty() => u,
        _ => {
            if let Ok(mut c) = CACHE.lock() { c.by_key.insert(key, State::Failed); }
            return None;
        }
    };
    if let Ok(mut c) = CACHE.lock() { c.by_key.insert(key, State::Pending); }

    let path = disk_path(key);
    if let Some(p) = path.as_ref() {
        if p.exists() {
            if let Ok(bytes) = std::fs::read(p) {
                let _ = CHAN.tx.send((key, Ok(bytes)));
                return None;
            }
        }
    }
    // Route through the single download worker; one HTTP fetch in flight
    // at a time. Avoids spawning hundreds of threads when a big fight lands.
    let _ = CHAN.req_tx.send((key, url, path));
    None
}

/// Process any completed downloads. Must be called from the imgui
/// thread (the only place we can safely touch the D3D11 device).
/// Bounded to `MAX_UPLOADS_PER_FRAME` to keep GPU work steady — Wine
/// has crashed when too many SRVs are created in a single frame.
pub fn drain_pending() {
    let device = match arcdps::d3d11_device() {
        Some(d) => d,
        None => return,
    };
    let Ok(rx) = CHAN.rx.lock() else { return };
    let mut uploaded = 0usize;
    loop {
        if uploaded >= MAX_UPLOADS_PER_FRAME { break; }
        let (key, result) = match rx.try_recv() {
            Ok(v) => v,
            Err(_) => break,
        };
        uploaded += 1;
        let new_state = match result {
            Ok(bytes) => match unsafe { upload(&device, &bytes) } {
                Ok((srv, aspect)) => {
                    let ptr = srv.as_raw() as usize;
                    let mut c = match CACHE.lock() { Ok(c) => c, Err(_) => continue };
                    c._srvs.push(srv);
                    State::Ready { ptr, aspect }
                }
                Err(e) => {
                    log::warn!("axipulse icon: upload failed for {:?}: {e}", key);
                    State::Failed
                }
            },
            Err(e) => {
                log::warn!("axipulse icon: download failed for {:?}: {e}", key);
                State::Failed
            }
        };
        if let Ok(mut c) = CACHE.lock() { c.by_key.insert(key, new_state); }
    }
}

fn disk_path(key: IconKey) -> Option<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")?;
    let kind = match key.kind { IconKind::Skill => "skill", IconKind::Buff => "buff" };
    let mut p = PathBuf::from(root);
    p.push("Axipulse"); p.push("icons");
    p.push(format!("{kind}_{}.png", key.id));
    Some(p)
}

unsafe fn upload(
    device: &ID3D11Device,
    bytes: &[u8],
) -> Result<(ID3D11ShaderResourceView, f32), Box<dyn std::error::Error>> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (w, h) = (img.width(), img.height());
    let aspect = if h > 0 { w as f32 / h as f32 } else { 1.0 };
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
    Ok((srv, aspect))
}
