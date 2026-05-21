use std::fs;
use tempfile::TempDir;

#[test]
fn extracts_zip_into_target_dir() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("test.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
    zip.start_file::<_, ()>("hello.txt", zip::write::SimpleFileOptions::default()).unwrap();
    use std::io::Write;
    zip.write_all(b"world").unwrap();
    zip.finish().unwrap();

    let out = tmp.path().join("out");
    arcdps_axipulse::ei_bundle::extract_zip(&zip_path, &out).unwrap();
    assert_eq!(fs::read_to_string(out.join("hello.txt")).unwrap(), "world");
}

#[test]
fn install_writes_marker_and_skips_when_already_installed() {
    let tmp = TempDir::new().unwrap();
    let install_root = tmp.path().join("install");

    let zip_path = tmp.path().join("ei.zip");
    {
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        zip.start_file::<_, ()>("GuildWars2EliteInsights-CLI.exe",
            zip::write::SimpleFileOptions::default()).unwrap();
        use std::io::Write;
        zip.write_all(b"dummy").unwrap();
        zip.finish().unwrap();
    }
    let bytes = fs::read(&zip_path).unwrap();

    arcdps_axipulse::ei_bundle::install_from_bytes(&bytes, "0.1.0", &install_root).unwrap();
    assert!(install_root.join("eicli").join("GuildWars2EliteInsights-CLI.exe").exists());
    assert_eq!(fs::read_to_string(install_root.join("eicli-version.txt")).unwrap(), "0.1.0");

    fs::remove_file(install_root.join("eicli").join("GuildWars2EliteInsights-CLI.exe")).unwrap();
    arcdps_axipulse::ei_bundle::install_from_bytes(&bytes, "0.1.0", &install_root).unwrap();
    assert!(!install_root.join("eicli").join("GuildWars2EliteInsights-CLI.exe").exists());
}
