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
