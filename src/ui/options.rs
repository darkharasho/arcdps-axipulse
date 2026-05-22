#![cfg(windows)]
//! arcdps options pane integration: a checkbox in the standard window
//! list to toggle the AxiPulse overlay (which contains Pulse + Timeline
//! tabs in a single window), plus a hotkey row in the per-addon
//! options pane.

use arcdps::imgui::Ui;

use crate::config::Config;
use crate::plugin::{binding_in_progress, cancel_binding, request_bind, BindingTarget};

pub fn render_window_checkboxes(ui: &Ui, config: &mut Config) -> bool {
    let mut changed = false;
    let mut show = config.show_pulse;
    if ui.checkbox("AxiPulse", &mut show) {
        config.show_pulse = show;
        changed = true;
    }
    changed
}

pub fn render_options_end(ui: &Ui, config: &mut Config) {
    ui.text("AxiPulse");
    ui.separator();
    let mut dirty = false;
    dirty |= render_hotkey_row(
        ui,
        "Toggle visibility",
        BindingTarget::ToggleVisibility,
        &mut config.toggle_visibility_hotkey,
    );
    ui.text_disabled("Click \"Set\" then press the chord. \"Clear\" disables the hotkey.");

    ui.separator();
    let mut show_notif = config.show_notifications;
    if ui.checkbox("Show parse notifications", &mut show_notif) {
        config.show_notifications = show_notif;
        dirty = true;
    }
    ui.text_disabled(
        "Transparent toast that flashes when a new log starts parsing and \
         when it finishes — works even with the main window hidden.",
    );

    if dirty { config.save(); }
}

fn render_hotkey_row(
    ui: &Ui,
    label: &str,
    target: BindingTarget,
    binding: &mut String,
) -> bool {
    let is_listening = binding_in_progress() == Some(target);
    let display = if is_listening {
        "Press a key...".to_string()
    } else if binding.is_empty() {
        "(unbound)".to_string()
    } else {
        binding.clone()
    };

    ui.text(format!("{label}:"));
    ui.same_line();
    let _w = ui.push_item_width(160.0);
    ui.label_text(format!("##hk-{label}"), &display);
    drop(_w);
    ui.same_line();

    let mut dirty = false;
    if is_listening {
        if ui.button(format!("Cancel##{label}")) {
            cancel_binding();
        }
    } else if ui.button(format!("Set##{label}")) {
        request_bind(target);
    }
    ui.same_line();
    if ui.button(format!("Clear##{label}")) {
        binding.clear();
        dirty = true;
    }
    dirty
}
