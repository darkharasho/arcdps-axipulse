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

    ui.separator();
    ui.text_disabled("Combat log folder");
    let detected = crate::config::default_cbtlogs()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(could not detect)".into());
    let effective = if config.cbtlogs_path.is_empty() {
        detected.clone()
    } else {
        config.cbtlogs_path.clone()
    };
    let exists = std::path::Path::new(&effective).is_dir();
    let status_color = if exists { [0.40, 0.92, 0.55, 1.0] } else { [1.00, 0.40, 0.40, 1.0] };
    let status = if exists { "found" } else { "not found" };
    ui.text_disabled(format!("Auto-detected: {detected}"));
    ui.text("Override path:");
    ui.same_line();
    let _w = ui.push_item_width(420.0);
    if ui.input_text("##cbtlogs-path", &mut config.cbtlogs_path)
        .hint("leave blank to use auto-detected path")
        .build()
    {
        dirty = true;
    }
    drop(_w);
    ui.text_colored(status_color, format!("Current: {effective} ({status})"));
    ui.text_disabled(
        "AxiPulse needs to read arcdps's combat logs. \
         Changes take effect on next GW2 launch."
    );

    ui.separator();
    ui.text_disabled("Updates");
    if ui.checkbox("Check for updates on startup", &mut config.auto_update_check) {
        dirty = true;
    }

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
