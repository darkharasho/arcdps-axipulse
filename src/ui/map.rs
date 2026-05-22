#![cfg(windows)]
//! WvW combat-replay map view. MVP: static final-frame render of
//! squad positions on top of tile background + landmark pins.

use arcdps::imgui::Ui;

use crate::derived::Derived;
use crate::ei_model::EiJson;

pub fn render_content(ui: &Ui, _json: &EiJson, _idx: usize, _derived: &Derived) {
    ui.text_disabled("Map view — coming up.");
}
