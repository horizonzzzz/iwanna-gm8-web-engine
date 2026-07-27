use std::fs;

use iwm_parser::models::ResourceIndex;
use iwm_parser::resource_export::export_external_audio_sidecars;
use iwm_parser::{LoweredLogicEntry, LoweredLogicExpr, LoweredLogicFile, LoweredLogicStatement};

#[test]
fn export_external_audio_copies_only_referenced_valid_sidecars() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    let output = temp.path().join("output");
    fs::create_dir_all(input.join("Music")).unwrap();
    fs::write(input.join("Music").join("Track.ogg"), b"OggSvalid").unwrap();
    fs::write(input.join("Music").join("unused.ogg"), b"OggSvalid").unwrap();
    let logic = LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![LoweredLogicEntry {
            block_id: "script:0".into(),
            statements: vec![LoweredLogicStatement::FunctionCall {
                name: "SS_LoadSound".into(),
                args: vec![LoweredLogicExpr::LiteralText("track.ogg".into())],
            }],
        }],
    };
    let mut resources = ResourceIndex {
        sprites: vec![],
        backgrounds: vec![],
        sounds: vec![],
        fonts: vec![],
        paths: vec![],
    };

    let warnings = export_external_audio_sidecars(&input, &output, &logic, &mut resources).unwrap();

    assert!(warnings.is_empty());
    assert_eq!(resources.sounds.len(), 1);
    assert_eq!(resources.sounds[0].name, "music/track.ogg");
    assert_eq!(
        fs::read(output.join(&resources.sounds[0].file_path)).unwrap(),
        b"OggSvalid"
    );
}
