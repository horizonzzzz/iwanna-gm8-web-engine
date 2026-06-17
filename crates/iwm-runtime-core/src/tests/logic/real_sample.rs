use super::*;

#[test]
fn real_sample_s_key_savepoint_writes_save_file_and_spawns_feedback() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
        host.input.clear_transitions();
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);

    core.reload_room(143).unwrap();
    core.globals
        .insert("global.difficulty".into(), RuntimeValue::Number(0.0));
    {
        let room = core.current_room.as_mut().unwrap();
        let savepoint = room
            .instances
            .iter()
            .find(|instance| {
                instance.object_name.eq_ignore_ascii_case("savePoint") && instance.alive
            })
            .cloned()
            .expect("sampleroom01 should include savePoint");
        let player = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name.eq_ignore_ascii_case("player") && instance.alive)
            .expect("sampleroom01 should include a live player");
        player.x = savepoint.x - savepoint.origin_x as f64
            + savepoint.bbox_left as f64
            + player.origin_x as f64
            - player.bbox_left as f64;
        player.y = savepoint.y - savepoint.origin_y as f64
            + savepoint.bbox_top as f64
            + player.origin_y as f64
            - player.bbox_top as f64;
        player.previous_x = player.x;
        player.previous_y = player.y;
        assert!(
            crate::helpers::collides_with_instance_at(
                player,
                player.x,
                player.y,
                &savepoint,
                None,
                |_| true
            ),
            "test setup should overlap player and savePoint"
        );
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x53),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let save_bytes = host
        .files
        .read(Path::new("save1"))
        .expect("S savepoint should write save1");
    assert!(
        save_bytes.len() >= 10,
        "save1 should contain room/player/difficulty bytes, got {save_bytes:?}"
    );

    let room = core.current_room().unwrap();
    for expected in ["object808", "object809", "object819"] {
        assert!(
            room.instances
                .iter()
                .any(|instance| instance.object_name.eq_ignore_ascii_case(expected)),
            "save feedback should create {expected}"
        );
    }
    assert!(
        !room
            .instances
            .iter()
            .any(
                |instance| instance.object_name.eq_ignore_ascii_case("savePoint") && instance.alive
            ),
        "activated savePoint should be hidden until its respawn helper fires"
    );

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x53),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
    for _ in 0..81 {
        core.tick(&mut host).unwrap();
        host.input.clear_transitions();
    }

    let room = core.current_room().unwrap();
    assert!(
        room.instances
            .iter()
            .any(
                |instance| instance.object_name.eq_ignore_ascii_case("savePoint") && instance.alive
            ),
        "savePoint should reappear after package-owned alarm helper recreates object id 5"
    );
    assert!(
        !room
            .instances
            .iter()
            .any(
                |instance| instance.object_name.eq_ignore_ascii_case("object819") && instance.alive
            ),
        "respawn helper should destroy itself after recreating savePoint"
    );
    assert!(
        core.diagnostics().iter().all(|diagnostic| {
            diagnostic.code != "runtime-unsupported-function"
                && diagnostic.code != "runtime-unsupported-expression"
        }),
        "savepoint path should not emit unsupported runtime diagnostics: {:?}",
        core.diagnostics()
    );
}

#[test]
fn real_sample_death_feedback_waits_for_reset_before_room_reload() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let snd_death_id = package
        .resources
        .sounds
        .iter()
        .find(|sound| sound.name.eq_ignore_ascii_case("sndDeath"))
        .map(|sound| sound.id as i32)
        .expect("sample package should include sndDeath");

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..2 {
        core.tick(&mut host).unwrap();
        host.input.clear_transitions();
    }
    core.reload_room(151).unwrap();
    core.render(&mut host).unwrap();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    for _ in 0..90 {
        core.tick(&mut host).unwrap();
        host.input.set_button_state(
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: true,
                just_pressed: false,
                just_released: false,
            },
        );
        if core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .any(|instance| instance.object_name.eq_ignore_ascii_case("GAMEOVER") && instance.alive)
        {
            break;
        }
    }

    let room = core.current_room().unwrap();
    assert_eq!(room.room_id, 151);
    assert!(
        room.instances.iter().any(
            |instance| instance.object_name.eq_ignore_ascii_case("GAMEOVER") && instance.alive
        ),
        "expected GAMEOVER after death, snapshot={:?}, live player={:?}, recent diagnostics={:?}",
        core.snapshot().player,
        room.instances
            .iter()
            .find(|instance| crate::helpers::is_player_instance(instance))
            .map(|instance| (
                instance.x,
                instance.y,
                instance.hspeed,
                instance.vspeed,
                instance.alive
            )),
        core.diagnostics().iter().rev().take(8).collect::<Vec<_>>()
    );
    assert!(
        host.audio
            .played
            .contains(&(snd_death_id, iwm_runtime_host::RuntimeSoundMode::Once)),
        "expected sndDeath id {snd_death_id}, played sounds: {:?}",
        host.audio.played
    );
    assert!(room.instances.iter().any(|instance| {
        instance.object_name.eq_ignore_ascii_case("bloodEmitter2") && instance.alive
    }));
    assert!(!room
        .instances
        .iter()
        .any(|instance| crate::helpers::is_player_instance(instance) && instance.alive));

    host.input.clear_transitions();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        },
    );
    let mut first_blood = None;
    for _ in 0..12 {
        core.tick(&mut host).unwrap();
        first_blood = core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.object_name.eq_ignore_ascii_case("blood2") && instance.alive)
            .cloned();
        if first_blood.is_some() {
            break;
        }
    }
    let first_blood = first_blood.expect("expected blood2 particles after bloodEmitter2 step");
    core.tick(&mut host).unwrap();
    let blood_after_motion = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.runtime_id == first_blood.runtime_id)
        .cloned()
        .expect("expected blood2 particle to remain addressable after one tick");
    assert!(
        blood_after_motion.x != first_blood.x
            || blood_after_motion.y != first_blood.y
            || !blood_after_motion.alive,
        "blood2 should move or collide after spawn, before={:?}, after={:?}",
        (
            first_blood.x,
            first_blood.y,
            first_blood.hspeed,
            first_blood.vspeed
        ),
        (
            blood_after_motion.x,
            blood_after_motion.y,
            blood_after_motion.hspeed,
            blood_after_motion.vspeed,
            blood_after_motion.alive
        )
    );
    for _ in 0..3 {
        core.tick(&mut host).unwrap();
    }
    assert_eq!(core.snapshot().room_id, Some(151));

    let reset_key = core
        .globals
        .get("global.restartbutton")
        .or_else(|| core.globals.get("global.resetbutton"))
        .and_then(crate::helpers::as_number)
        .map(|value| value.round() as u16)
        .unwrap_or(0x52);
    host.input.set_button_state(
        RuntimeButton::Keyboard(reset_key),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    assert_eq!(room.room_id, 151);
    assert!(room
        .instances
        .iter()
        .any(|instance| crate::helpers::is_player_instance(instance) && instance.alive));
    assert!(!room
        .instances
        .iter()
        .any(|instance| instance.object_name.eq_ignore_ascii_case("GAMEOVER") && instance.alive));
    assert!(!room.instances.iter().any(|instance| {
        instance.object_name.eq_ignore_ascii_case("bloodEmitter2") && instance.alive
    }));
}

#[test]
fn real_sample_second_shift_press_lacks_bootstrap_globals_after_manual_room_reload() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(step_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "object:0:event:3:0")
        {
            if let Some(jump_cond_index) = step_entry.statements.iter().position(|statement| {
                matches!(
                    statement,
                    LoweredLogicStatement::Conditional {
                        condition: LoweredLogicExpr::Call { name, args },
                        ..
                    } if name == "keyboard_check_pressed"
                        && matches!(
                            args.first(),
                            Some(LoweredLogicExpr::MemberAccess { target, member })
                                if member == "jumpbutton"
                                    && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                        )
                )
            }) {
                step_entry.statements.insert(
                    jump_cond_index + 1,
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("debug_after_jump_cond_vspeed".into()),
                        value: LoweredLogicExpr::Identifier("vspeed".into()),
                    },
                );
            }
            step_entry.statements.insert(
                0,
                LoweredLogicStatement::Conditional {
                    condition: LoweredLogicExpr::Call {
                        name: "keyboard_check_pressed".into(),
                        args: vec![LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                            member: "jumpbutton".into(),
                        }],
                    },
                    then_branch: vec![LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("debug_step_jump_pressed".into()),
                        value: LoweredLogicExpr::LiteralBool(true),
                    }],
                    else_branch: vec![],
                },
            );
        }

        if let Some(player_jump_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "script:11")
        {
            player_jump_entry.statements.insert(
                0,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_player_jump_called".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
            );
            player_jump_entry.statements.insert(
                1,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_ground_block".into()),
                    value: LoweredLogicExpr::Call {
                        name: "place_meeting".into(),
                        args: vec![
                            LoweredLogicExpr::Identifier("x".into()),
                            LoweredLogicExpr::BinaryExpr {
                                op: "+".into(),
                                left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                            },
                            LoweredLogicExpr::Identifier("block".into()),
                        ],
                    },
                },
            );
            player_jump_entry.statements.insert(
                2,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_ground_solidblock".into()),
                    value: LoweredLogicExpr::Call {
                        name: "place_meeting".into(),
                        args: vec![
                            LoweredLogicExpr::Identifier("x".into()),
                            LoweredLogicExpr::BinaryExpr {
                                op: "+".into(),
                                left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                            },
                            LoweredLogicExpr::Identifier("solidblock".into()),
                        ],
                    },
                },
            );
            player_jump_entry.statements.insert(
                3,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_pre_djump".into()),
                    value: LoweredLogicExpr::Identifier("djump".into()),
                },
            );
            player_jump_entry.statements.insert(
                4,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_pre_onPlatform".into()),
                    value: LoweredLogicExpr::Identifier("onPlatform".into()),
                },
            );
            if let Some(LoweredLogicStatement::Conditional { then_branch, .. }) =
                player_jump_entry.statements.get_mut(5)
            {
                if let Some(LoweredLogicStatement::Conditional {
                    then_branch: jump_ground_branch,
                    else_branch: jump_ground_else,
                    ..
                }) = then_branch.get_mut(0)
                {
                    jump_ground_branch.insert(
                        0,
                        LoweredLogicStatement::Assignment {
                            target: LoweredLogicExpr::Identifier(
                                "debug_ground_branch_taken".into(),
                            ),
                            value: LoweredLogicExpr::LiteralBool(true),
                        },
                    );
                    if let Some(LoweredLogicStatement::Conditional {
                        then_branch: jump_air_branch,
                        ..
                    }) = jump_ground_else.get_mut(0)
                    {
                        jump_air_branch.insert(
                            0,
                            LoweredLogicStatement::Assignment {
                                target: LoweredLogicExpr::Identifier(
                                    "debug_air_branch_taken".into(),
                                ),
                                value: LoweredLogicExpr::LiteralBool(true),
                            },
                        );
                    }
                }
            }
        }
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);

    core.reload_room(143).unwrap();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        let snapshot = core.snapshot();
        if snapshot
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }
    assert!(core
        .snapshot()
        .player
        .as_ref()
        .map(|player| player.jump.grounded)
        .unwrap_or(false));

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
    core.tick(&mut host).unwrap();

    host.input.clear_transitions();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        },
    );
    for _ in 0..180 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let after_second = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| crate::helpers::is_player_instance(instance))
        .unwrap();

    assert_eq!(
        core.globals.get("global.grav"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert_eq!(
        after_second.vars.get("debug_step_jump_pressed"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_player_jump_called"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_ground_block"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_ground_branch_taken"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(
        after_second.vspeed < 0.0,
        "second jump should produce upward vspeed once bootstrap globals exist, got {:?}",
        after_second.vspeed
    );
}
#[test]
fn real_sample_step_events_alone_show_second_shift_playerjump_effect() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(player_jump_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "script:11")
        {
            player_jump_entry.statements.insert(
                0,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_player_jump_called".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
            );
        }
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);

    core.reload_room(143).unwrap();
    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.execute_lowered_step_events(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| crate::helpers::is_player_instance(instance))
        .unwrap();

    assert_eq!(
        player.vars.get("debug_player_jump_called"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(
        player.vspeed < 0.0,
        "step events alone should apply upward jump velocity, got {:?}",
        player.vspeed
    );
}

#[test]
fn real_sample_step_events_spawn_bullet_and_play_shoot_sound_on_z_press() {
    let Some(package) = real_sample_package() else {
        return;
    };

    let shoot_sound_id = package
        .resources
        .sounds
        .iter()
        .find(|sound| sound.name.eq_ignore_ascii_case("sndShoot"))
        .map(|sound| sound.id as i32)
        .expect("sample package should include sndShoot");

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);
    assert_eq!(
        core.globals.get("global.shotbutton"),
        Some(&RuntimeValue::Number(0x5A as f64))
    );

    core.reload_room(143).unwrap();
    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    let bullet_count_before = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name.eq_ignore_ascii_case("bullet"))
        .count();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x5A),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.execute_lowered_step_events(&mut host).unwrap();

    let bullet_count_after = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name.eq_ignore_ascii_case("bullet"))
        .count();

    assert_eq!(
        bullet_count_after,
        bullet_count_before + 1,
        "Z press should spawn one bullet instance"
    );
    assert!(
        host.audio
            .played
            .contains(&(shoot_sound_id, iwm_runtime_host::RuntimeSoundMode::Once)),
        "Z press should dispatch sndShoot once, got {:?}",
        host.audio.played
    );
}

#[test]
fn real_sample_spawned_bullet_moves_and_can_see_forward_block_collision() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(bullet_step_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "object:2:event:3:0")
        {
            bullet_step_entry.statements.insert(
                0,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_forward_block".into()),
                    value: LoweredLogicExpr::Call {
                        name: "place_meeting".into(),
                        args: vec![
                            LoweredLogicExpr::BinaryExpr {
                                op: "+".into(),
                                left: Box::new(LoweredLogicExpr::Identifier("x".into())),
                                right: Box::new(LoweredLogicExpr::Identifier("hspeed".into())),
                            },
                            LoweredLogicExpr::Identifier("y".into()),
                            LoweredLogicExpr::Identifier("block".into()),
                        ],
                    },
                },
            );
        }
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }

    core.reload_room(143).unwrap();
    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x5A),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();
    host.input.clear_transitions();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x5A),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        },
    );

    let bullet_after_spawn = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_name.eq_ignore_ascii_case("bullet"))
        .cloned()
        .expect("expected spawned bullet");

    core.tick(&mut host).unwrap();

    let bullet_after_step = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.runtime_id == bullet_after_spawn.runtime_id)
        .cloned()
        .expect("expected bullet after step");

    let room = core.current_room().unwrap();
    let player_like = room
        .instances
        .iter()
        .filter(|instance| {
            instance.object_name.eq_ignore_ascii_case("player")
                || instance.object_name.eq_ignore_ascii_case("player2")
        })
        .map(|instance| {
            (
                instance.object_name.clone(),
                instance.alive,
                instance.vars.get("image_xscale").cloned(),
                instance.x,
                instance.y,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        bullet_after_spawn.hspeed.abs(),
        16.0,
        "spawned bullet should inherit 16px horizontal speed, bullet={:?}, players={:?}",
        (
            bullet_after_spawn.runtime_id,
            bullet_after_spawn.x,
            bullet_after_spawn.y,
            bullet_after_spawn.hspeed,
            bullet_after_spawn.vars.clone()
        ),
        player_like
    );
    assert!(
        bullet_after_step.x != bullet_after_spawn.x || !bullet_after_step.alive,
        "bullet should either move or destroy itself after step, spawn={:?} step={:?}",
        (
            bullet_after_spawn.x,
            bullet_after_spawn.y,
            bullet_after_spawn.hspeed,
            bullet_after_spawn.alive
        ),
        (
            bullet_after_step.x,
            bullet_after_step.y,
            bullet_after_step.hspeed,
            bullet_after_step.alive
        )
    );
}
