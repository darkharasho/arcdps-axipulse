use arcdps_axipulse::ei_settings::{generate_ei_conf, EiSettings};

#[test]
fn includes_required_axipulse_flags() {
    let settings = EiSettings::default();
    let conf = generate_ei_conf(&settings, "C:\\out");

    assert!(conf.contains("SaveOutJSON=True"));
    assert!(conf.contains("CompressRaw=True"));
    assert!(conf.contains("SaveOutHTML=False"));
    assert!(conf.contains("DetailledWvW=True"));
    assert!(conf.contains("RawTimelineArrays=True"));
    assert!(conf.contains("ComputeDamageModifiers=True"));
    assert!(conf.contains("ParsePhases=True"));
    assert!(conf.contains("ParseCombatReplay=True"));
    assert!(conf.contains("UploadToDPSReports=False"));
    assert!(conf.contains("CustomTooShort=2200"));
    assert!(conf.contains("OutLocation=C:\\out"));
}

#[test]
fn boolean_flags_serialise_as_True_False() {
    let mut settings = EiSettings::default();
    settings.detailled_wvw = false;
    let conf = generate_ei_conf(&settings, "/tmp");
    assert!(conf.contains("DetailledWvW=False"));
    assert!(!conf.contains("DetailledWvW=false"));
}
