#![cfg(windows)]
//! arcdps options pane integration: a checkbox in the standard window
//! list to toggle the AxiPulse overlay (which contains Pulse + Timeline
//! tabs in a single window).

use arcdps::imgui::Ui;

use crate::config::Config;

pub fn render_window_checkboxes(ui: &Ui, config: &mut Config) -> bool {
    let mut changed = false;
    let mut show = config.show_pulse;
    if ui.checkbox("AxiPulse", &mut show) {
        config.show_pulse = show;
        changed = true;
    }
    changed
}
