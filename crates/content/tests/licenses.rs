//! No audio file enters the repository or the build without a row in
//! `LICENSES.md` (PLANNING-BRIEF 0.5).
//!
//! `packaging/package-web.sh` refuses to package such a file; this test looks
//! at the same directories so the suite fails before packaging does.

use std::path::{Path, PathBuf};

const AUDIO: [&str; 9] = ["mp3", "ogg", "oga", "wav", "flac", "m4a", "aac", "opus", "weba"];

fn root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).canonicalize().unwrap()
}

fn audio_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd {
        let p = e.unwrap().path();
        if p.is_dir() {
            audio_files(&p, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| AUDIO.contains(&e.to_ascii_lowercase().as_str()))
        {
            out.push(p);
        }
    }
}

#[test]
fn no_audio_ships_without_a_license_row() {
    let root = root();
    let licenses = std::fs::read_to_string(root.join("LICENSES.md")).unwrap();
    let mut found = Vec::new();
    for d in ["web", "data"] {
        audio_files(&root.join(d), &mut found);
    }
    let unlicensed: Vec<String> = found
        .iter()
        .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
        .filter(|rel| !licenses.lines().any(|l| l.starts_with('|') && l.contains(&format!("`{rel}`"))))
        .collect();
    assert!(
        unlicensed.is_empty(),
        "\nthese audio files have no row in LICENSES.md: {unlicensed:?}\n\
         Do not add a row to make this pass: an audio file entering the build is Sam's decision.\n"
    );
}
