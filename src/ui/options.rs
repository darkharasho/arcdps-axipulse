#![cfg(windows)]
//! arcdps options pane integration: a checkbox in the standard window
//! list to toggle the Pulse overlay.

use arcdps::imgui::Ui;

use crate::config::Config;

pub fn render_window_checkboxes(ui: &Ui, config: &mut Config) -> bool {
    let mut changed = false;
    let mut show = config.show_pulse;
    if ui.checkbox("Pulse", &mut show) {
        config.show_pulse = show;
        changed = true;
    }
    changed
}
