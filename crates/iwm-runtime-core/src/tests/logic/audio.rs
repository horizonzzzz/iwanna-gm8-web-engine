use super::*;

#[test]
fn core_dispatches_sound_play_identifier_to_audio_host() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "sndJump".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "sound_play".into(),
            args: vec![LoweredLogicExpr::Identifier("sndJump".into())],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.played, vec![(42, RuntimeSoundMode::Once)]);
    assert!(core.diagnostics().iter().all(|entry| {
        entry.code != "runtime-unsupported-function"
            || !entry.message.contains("function=sound_play")
    }));
}

#[test]
fn core_dispatches_sound_loop_identifier_to_audio_host() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "sndJump".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "sound_loop".into(),
            args: vec![LoweredLogicExpr::Identifier("sndJump".into())],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.played, vec![(42, RuntimeSoundMode::Loop)]);
}

#[test]
fn core_dispatches_sound_stop_identifier_to_audio_host() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "sndJump".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "sound_stop".into(),
            args: vec![LoweredLogicExpr::Identifier("sndJump".into())],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.stopped, vec![42]);
}

#[test]
fn core_evaluates_sound_isplaying_in_conditionals() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "track01".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "sound_isplaying".into(),
                args: vec![LoweredLogicExpr::Identifier("track01".into())],
            },
            then_branch: vec![],
            else_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "sound_loop".into(),
                args: vec![LoweredLogicExpr::Identifier("track01".into())],
            }],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();
    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.played, vec![(42, RuntimeSoundMode::Loop)]);
    assert!(core.diagnostics().iter().all(|entry| {
        entry.code != "runtime-unsupported-function"
            || !entry.message.contains("function=sound_isplaying")
    }));
}

#[test]
fn core_dispatches_sound_stop_all_to_audio_host() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "track01".into();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "sound_loop".into(),
                args: vec![LoweredLogicExpr::Identifier("track01".into())],
            },
            LoweredLogicStatement::FunctionCall {
                name: "sound_stop_all".into(),
                args: vec![],
            },
        ],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.stopped_all_count, 1);
    assert!(!host.audio.is_sound_playing(42).unwrap());
}
