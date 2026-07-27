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

#[test]
fn core_stops_one_shots_but_keeps_bgm_loop_when_restart_button_resets_room() {
    let mut package = sample_package();
    package.manifest.zero_uninitialized_vars = true;
    package.resources.sounds[0].id = 7;
    package.resources.sounds[0].name = "track01".into();
    let mut death_sound = package.resources.sounds[0].clone();
    death_sound.id = 42;
    death_sound.name = "sndDeath".into();
    package.resources.sounds.push(death_sound);
    // Start the BGM loop and the death jingle exactly once, guarded by a
    // global so the rebuilt room does not replay them after the reset.
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::UnaryExpr {
                op: "!".into(),
                child: Box::new(LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "deathsoundplayed".into(),
                }),
            },
            then_branch: vec![
                LoweredLogicStatement::FunctionCall {
                    name: "sound_loop".into(),
                    args: vec![LoweredLogicExpr::Identifier("track01".into())],
                },
                LoweredLogicStatement::FunctionCall {
                    name: "sound_play".into(),
                    args: vec![LoweredLogicExpr::Identifier("sndDeath".into())],
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                        member: "deathsoundplayed".into(),
                    },
                    value: LoweredLogicExpr::LiteralNumber(1.0),
                },
            ],
            else_branch: vec![],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();
    assert_eq!(
        host.audio.played,
        vec![(7, RuntimeSoundMode::Loop), (42, RuntimeSoundMode::Once)]
    );
    assert!(host.audio.is_sound_playing(7).unwrap());
    assert!(host.audio.is_sound_playing(42).unwrap());

    host.input.set_button_state(
        RuntimeButton::Restart,
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    // GM8 keeps sounds across restart; only the death one-shot is cut so the
    // looping BGM survives for the game's own sound_isplaying guards.
    assert_eq!(host.audio.stopped_all_count, 0);
    assert_eq!(host.audio.stopped, vec![42]);
    assert!(!host.audio.is_sound_playing(42).unwrap());
    assert!(host.audio.is_sound_playing(7).unwrap());
    assert_eq!(
        host.audio.played,
        vec![(7, RuntimeSoundMode::Loop), (42, RuntimeSoundMode::Once)]
    );
}

#[test]
fn core_stops_one_shots_but_keeps_bgm_loop_when_game_restart_reloads_first_room() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 7;
    package.resources.sounds[0].name = "track01".into();
    let mut death_sound = package.resources.sounds[0].clone();
    death_sound.id = 42;
    death_sound.name = "sndDeath".into();
    package.resources.sounds.push(death_sound);
    add_keyboard_block(
        &mut package,
        65,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "sound_loop".into(),
                args: vec![LoweredLogicExpr::Identifier("track01".into())],
            },
            LoweredLogicStatement::FunctionCall {
                name: "sound_play".into(),
                args: vec![LoweredLogicExpr::Identifier("sndDeath".into())],
            },
        ],
    );
    add_keyboard_block(
        &mut package,
        82,
        vec![LoweredLogicStatement::FunctionCall {
            name: "game_restart".into(),
            args: vec![],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    host.input.set_button_state(
        RuntimeButton::Keyboard(65),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();
    assert_eq!(
        host.audio.played,
        vec![(7, RuntimeSoundMode::Loop), (42, RuntimeSoundMode::Once)]
    );
    assert!(host.audio.is_sound_playing(7).unwrap());
    assert!(host.audio.is_sound_playing(42).unwrap());

    host.input
        .set_button_state(RuntimeButton::Keyboard(65), ButtonState::default());
    host.input.set_button_state(
        RuntimeButton::Keyboard(82),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.stopped_all_count, 0);
    assert_eq!(host.audio.stopped, vec![42]);
    assert!(!host.audio.is_sound_playing(42).unwrap());
    assert!(host.audio.is_sound_playing(7).unwrap());
    assert_eq!(
        host.audio.played,
        vec![(7, RuntimeSoundMode::Loop), (42, RuntimeSoundMode::Once)]
    );
}

#[test]
fn core_treats_uninitialized_sound_var_as_zero_when_enabled() {
    let mut package = sample_package();
    package.manifest.zero_uninitialized_vars = true;
    package.resources.sounds[0].id = 0;
    package.resources.sounds[0].name = "track01".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::UnaryExpr {
                op: "!".into(),
                child: Box::new(LoweredLogicExpr::Call {
                    name: "sound_isplaying".into(),
                    args: vec![LoweredLogicExpr::Identifier("stageBGM".into())],
                }),
            },
            then_branch: vec![
                LoweredLogicStatement::FunctionCall {
                    name: "sound_stop_all".into(),
                    args: vec![],
                },
                LoweredLogicStatement::FunctionCall {
                    name: "sound_loop".into(),
                    args: vec![LoweredLogicExpr::Identifier("stageBGM".into())],
                },
            ],
            else_branch: vec![],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(host.audio.stopped_all_count, 1);
    assert_eq!(host.audio.played, vec![(0, RuntimeSoundMode::Loop)]);
    assert!(host.audio.is_sound_playing(0).unwrap());
}
