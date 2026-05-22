//! WvW combat-replay map subsystem. Pure data + coord math (no I/O,
//! no D3D11). UI lives in `crate::ui::map`; texture cache lives in
//! `crate::ui::tile_cache`.

pub mod wvw;
pub mod tiles;
