use super::*;

use crate::RuntimeInstance;

fn move_real_sample_player_onto_savepoint(core: &mut RuntimeCore) {
    move_real_sample_player_onto_target(
        core,
        |instance| instance.object_name.eq_ignore_ascii_case("savePoint"),
        "savePoint",
    );
}

fn move_real_sample_player_onto_savepoint_at(core: &mut RuntimeCore, x: f64, y: f64) -> usize {
    move_real_sample_player_onto_target(
        core,
        |instance| {
            instance.object_name.eq_ignore_ascii_case("savePoint")
                && instance.x == x
                && instance.y == y
        },
        "savePoint at requested coordinates",
    )
}

fn move_real_sample_player_onto_target<F>(
    core: &mut RuntimeCore,
    mut matches_target: F,
    description: &str,
) -> usize
where
    F: FnMut(&RuntimeInstance) -> bool,
{
    let room = core.current_room.as_mut().unwrap();
    let target = room
        .instances
        .iter()
        .find(|instance| instance.alive && matches_target(instance))
        .cloned()
        .unwrap_or_else(|| panic!("room should include a live {description}"));
    let player = room
        .instances
        .iter_mut()
        .find(|instance| instance.object_name.eq_ignore_ascii_case("player") && instance.alive)
        .expect("room should include a live player");
    let (x, y) = overlapping_player_position(player, &target)
        .unwrap_or_else(|| panic!("test setup should find overlap for {description}"));
    player.x = x;
    player.y = y;
    player.previous_x = player.x;
    player.previous_y = player.y;
    assert!(
        crate::helpers::collides_with_instance_at(
            player,
            player.x,
            player.y,
            &target,
            None,
            |_| true
        ),
        "test setup should overlap player and {description}"
    );
    target.runtime_id
}

fn overlapping_player_position(
    player: &RuntimeInstance,
    target: &RuntimeInstance,
) -> Option<(f64, f64)> {
    let (left, top, right, bottom) = crate::helpers::bounds_at(target, target.x, target.y);
    for world_y in top..bottom {
        for world_x in left..right {
            for player_local_y in player.bbox_top..=player.bbox_bottom {
                for player_local_x in player.bbox_left..=player.bbox_right {
                    let x = (world_x + player.origin_x - player_local_x) as f64;
                    let y = (world_y + player.origin_y - player_local_y) as f64;
                    if crate::helpers::collides_with_instance_at(player, x, y, target, None, |_| {
                        true
                    }) {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

fn real_sample_room_id(core: &RuntimeCore, name: &str) -> usize {
    core.package
        .rooms
        .iter()
        .find(|room| room.name.eq_ignore_ascii_case(name))
        .map(|room| room.id)
        .unwrap_or_else(|| panic!("sample package should include room {name}"))
}

fn tick_real_sample_until_room(
    core: &mut RuntimeCore,
    host: &mut HeadlessHost,
    room_id: usize,
    name: &str,
) {
    for _ in 0..120 {
        if core.snapshot().room_id == Some(room_id) {
            return;
        }
        core.tick(host).unwrap();
        host.input.clear_transitions();
    }
    assert_eq!(
        core.snapshot().room_id,
        Some(room_id),
        "sample should reach {name}"
    );
}

fn tap_real_sample_jump(core: &mut RuntimeCore, host: &mut HeadlessHost) {
    let key = match core.globals.get("global.jumpbutton") {
        Some(RuntimeValue::Number(value)) => *value as u16,
        _ => 0x10,
    };
    press_real_sample_key(host, key);
    core.tick(host).unwrap();
    release_real_sample_key(host, key);
    host.input.clear_transitions();
}

fn enter_real_sample_difficulty_room(core: &mut RuntimeCore, host: &mut HeadlessHost) {
    let title_room = real_sample_room_id(core, "rTitle");
    let menu_room = real_sample_room_id(core, "rMenu");
    let difficulty_room = real_sample_room_id(core, "rSelectStage");

    if !matches!(
        core.snapshot().room_id,
        Some(room_id) if room_id == title_room || room_id == menu_room || room_id == difficulty_room
    ) {
        tick_real_sample_until_room(core, host, title_room, "rTitle");
    }
    if core.snapshot().room_id == Some(title_room) {
        tap_real_sample_jump(core, host);
        tick_real_sample_until_room(core, host, menu_room, "rMenu");
    }
    if core.snapshot().room_id == Some(menu_room) {
        let live_worlds = core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .filter(|instance| instance.object_name.eq_ignore_ascii_case("world") && instance.alive)
            .map(|instance| instance.runtime_id)
            .collect::<Vec<_>>();
        assert_eq!(
            live_worlds.len(),
            1,
            "package title flow should carry one persistent world into rMenu, got {live_worlds:?}"
        );
    }
    if core.snapshot().room_id == Some(menu_room) {
        tap_real_sample_jump(core, host);
        tick_real_sample_until_room(core, host, difficulty_room, "rSelectStage");
    }
    assert_eq!(core.snapshot().room_id, Some(difficulty_room));
    let live_worlds = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name.eq_ignore_ascii_case("world") && instance.alive)
        .map(|instance| instance.runtime_id)
        .collect::<Vec<_>>();
    assert_eq!(
        live_worlds.len(),
        1,
        "package menu flow should carry one persistent world into rSelectStage, got {live_worlds:?}; diagnostics={:?}",
        core.diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("world"))
            .collect::<Vec<_>>()
    );
}

fn select_real_sample_medium_difficulty(core: &mut RuntimeCore, host: &mut HeadlessHost) {
    enter_real_sample_difficulty_room(core, host);
    let stage_room = real_sample_room_id(core, "rStage01");

    move_real_sample_player_onto_target(
        core,
        |instance| {
            instance.object_name.eq_ignore_ascii_case("warpStart")
                && instance.vars.get("dif") == Some(&RuntimeValue::Number(0.0))
        },
        "medium difficulty warpStart",
    );

    core.tick(host).unwrap();
    host.input.clear_transitions();

    assert_eq!(
        core.globals.get("global.difficulty"),
        Some(&RuntimeValue::Number(0.0)),
        "difficulty should be selected through the package-owned warpStart collision; room={:?}, players={:?}, recent diagnostics={:?}",
        core.snapshot().room_id,
        core.current_room()
            .unwrap()
            .instances
            .iter()
            .filter(|instance| crate::helpers::is_player_instance(instance))
            .map(|instance| (
                instance.runtime_id,
                instance.alive,
                instance.x,
                instance.y,
                instance.hspeed,
                instance.vspeed,
                instance.vars.get("frozen"),
            ))
            .collect::<Vec<_>>(),
        core.diagnostics().iter().rev().take(12).collect::<Vec<_>>()
    );
    assert_eq!(core.snapshot().room_id, Some(stage_room));
}

#[test]
fn real_sample_r_select_stage_spike_reset_keeps_player_movable() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let difficulty_room = real_sample_room_id(&core, "rSelectStage");

    enter_real_sample_difficulty_room(&mut core, &mut host);
    move_real_sample_player_onto_target(
        &mut core,
        |instance| instance.hazard && instance.object_name.to_ascii_lowercase().contains("spike"),
        "rSelectStage spike hazard",
    );

    core.tick(&mut host).unwrap();
    host.input.clear_transitions();

    assert_eq!(core.snapshot().room_id, Some(difficulty_room));
    let x_before = core
        .snapshot()
        .player
        .as_ref()
        .map(|player| player.x)
        .expect("spike reset should recreate a live player");

    press_real_sample_key(&mut host, 0x27);
    core.tick(&mut host).unwrap();

    let player = core
        .snapshot()
        .player
        .expect("player should remain live after reset movement");
    assert!(
        player.x > x_before,
        "player should move right after rSelectStage spike reset; before={x_before}, after={}, diagnostics={:?}",
        player.x,
        core.diagnostics().iter().rev().take(12).collect::<Vec<_>>()
    );
}

#[test]
fn real_sample_bootstrap_sets_shift_jump_binding() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let title_room = real_sample_room_id(&core, "rTitle");

    tick_real_sample_until_room(&mut core, &mut host, title_room, "rTitle");

    assert_eq!(
        core.globals.get("global.jumpbutton"),
        Some(&RuntimeValue::Number(0x10 as f64))
    );
}

#[test]
fn real_sample_title_shift_enters_save_menu() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let title_room = real_sample_room_id(&core, "rTitle");
    let menu_room = real_sample_room_id(&core, "rMenu");

    tick_real_sample_until_room(&mut core, &mut host, title_room, "rTitle");
    tap_real_sample_jump(&mut core, &mut host);

    assert_eq!(
        core.snapshot().room_id,
        Some(menu_room),
        "title Shift should follow parsed room_goto(rMenu); globals={:?}, input_trace={:?}, instances={:?}, diagnostics={:?}",
        core.globals,
        core.snapshot().input_trace,
        core.current_room()
            .unwrap()
            .instances
            .iter()
            .map(|instance| (
                instance.object_name.clone(),
                instance.object_id,
                instance.alive,
                instance.vars.clone()
            ))
            .collect::<Vec<_>>(),
        core.diagnostics().iter().rev().take(12).collect::<Vec<_>>()
    );
}

#[test]
fn real_sample_new_game_starts_at_stage_spawn_and_writes_initial_save() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    select_real_sample_medium_difficulty(&mut core, &mut host);

    let snapshot = core.snapshot();
    let player = snapshot
        .player
        .as_ref()
        .expect("new game should create a live player at the first stage start");
    assert_eq!(snapshot.room_id, Some(147));
    assert_eq!((player.x, player.y), (913.0, 1175.0));
    let save_bytes = host.files.read(Path::new("save1")).unwrap();
    assert!(
        save_bytes.starts_with(&[0, 1, 47, 0, 9, 13, 0, 11, 75, 0, 0]),
        "new-game save should start with room/player/difficulty bytes, got {save_bytes:?}"
    );
}

fn release_real_sample_key(host: &mut HeadlessHost, key: u16) {
    host.input.set_button_state(
        RuntimeButton::Keyboard(key),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
}

fn press_real_sample_key(host: &mut HeadlessHost, key: u16) {
    host.input.set_button_state(
        RuntimeButton::Keyboard(key),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
}

#[test]
fn real_sample_room147_s_key_savepoint_respawns_at_activated_position() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    select_real_sample_medium_difficulty(&mut core, &mut host);
    let activated_runtime_id = move_real_sample_player_onto_savepoint_at(&mut core, 864.0, 1120.0);

    press_real_sample_key(&mut host, 0x53);
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    assert!(
        !room.instances.iter().any(|instance| {
            instance.runtime_id == activated_runtime_id
                && instance.object_name.eq_ignore_ascii_case("savePoint")
                && instance.alive
        }),
        "activated savePoint should be destroyed while its feedback/helper animation runs"
    );
    assert!(
        room.instances.iter().any(|instance| instance
            .object_name
            .eq_ignore_ascii_case("object819")
            && instance.alive
            && instance.x == 864.0
            && instance.y == 1120.0),
        "respawn helper should be created at the activated savePoint position"
    );

    release_real_sample_key(&mut host, 0x53);
    for _ in 0..81 {
        core.tick(&mut host).unwrap();
        host.input.clear_transitions();
    }

    let room = core.current_room().unwrap();
    assert!(
        room.instances.iter().any(|instance| {
            instance.runtime_id != activated_runtime_id
                && instance.object_name.eq_ignore_ascii_case("savePoint")
                && instance.alive
                && instance.x == 864.0
                && instance.y == 1120.0
        }),
        "a new live savePoint should reappear at the activated position; difficulty={:?}, matching instances={:?}, recent diagnostics={:?}",
        core.globals.get("global.difficulty"),
        room.instances
            .iter()
            .filter(|instance| instance.object_name.eq_ignore_ascii_case("savePoint")
                && instance.x == 864.0
                && instance.y == 1120.0)
            .map(|instance| (instance.runtime_id, instance.alive, instance.vars.get("saveTimer")))
            .collect::<Vec<_>>(),
        core.diagnostics().iter().rev().take(20).collect::<Vec<_>>()
    );
}

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

    select_real_sample_medium_difficulty(&mut core, &mut host);
    core.reload_room(143).unwrap();
    move_real_sample_player_onto_savepoint(&mut core);

    press_real_sample_key(&mut host, 0x53);
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

    release_real_sample_key(&mut host, 0x53);
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
fn real_sample_r_load_after_s_save_restores_saved_player_position() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    select_real_sample_medium_difficulty(&mut core, &mut host);
    core.reload_room(143).unwrap();
    move_real_sample_player_onto_savepoint(&mut core);
    let saved_position = core
        .snapshot()
        .player
        .as_ref()
        .map(|player| (player.x, player.y))
        .expect("test setup should have a player before saving");

    press_real_sample_key(&mut host, 0x53);
    core.tick(&mut host).unwrap();
    release_real_sample_key(&mut host, 0x53);
    host.input.clear_transitions();
    assert_eq!(
        core.globals.get("global.grav"),
        Some(&RuntimeValue::Number(0.0)),
        "S save should not mutate global.grav before restart; save1={:?}",
        host.files.read(Path::new("save1"))
    );

    press_real_sample_key(&mut host, 0x52);
    core.tick(&mut host).unwrap();
    release_real_sample_key(&mut host, 0x52);

    let snapshot = core.snapshot();
    let player = snapshot
        .player
        .as_ref()
        .expect("R load should leave a live player");
    assert_eq!(snapshot.room_id, Some(143));
    assert_eq!(
        (player.x, player.y),
        saved_position,
        "R load should restore the exact saved position; room={:?}, difficulty={:?}, grav={:?}, save1={:?}, live players={:?}, recent diagnostics={:?}",
        core.snapshot().room_id,
        core.globals.get("global.difficulty"),
        core.globals.get("global.grav"),
        host.files.read(Path::new("save1")),
        core.current_room()
            .unwrap()
            .instances
            .iter()
            .filter(|instance| crate::helpers::is_player_instance(instance))
            .map(|instance| (instance.object_name.clone(), instance.runtime_id, instance.alive, instance.x, instance.y))
            .collect::<Vec<_>>(),
        core.diagnostics().iter().rev().take(12).collect::<Vec<_>>()
    );
}

#[test]
fn real_sample_r_load_after_s_save_keeps_shift_jump_bound() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    select_real_sample_medium_difficulty(&mut core, &mut host);
    core.reload_room(143).unwrap();
    move_real_sample_player_onto_savepoint(&mut core);

    press_real_sample_key(&mut host, 0x53);
    core.tick(&mut host).unwrap();
    release_real_sample_key(&mut host, 0x53);
    host.input.clear_transitions();

    press_real_sample_key(&mut host, 0x52);
    core.tick(&mut host).unwrap();
    release_real_sample_key(&mut host, 0x52);
    core.tick(&mut host).unwrap();
    host.input.clear_transitions();

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
    assert!(
        core.snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false),
        "R load should leave the player able to land before testing jump; room={:?}; players={:?}",
        core.snapshot().room_id,
        core.current_room()
            .unwrap()
            .instances
            .iter()
            .filter(|instance| crate::helpers::is_player_instance(instance))
            .map(|instance| (
                instance.runtime_id,
                instance.instance_id,
                instance.alive,
                instance.x,
                instance.y,
                instance.vspeed,
                instance.vars.clone()
            ))
            .collect::<Vec<_>>()
    );

    press_real_sample_key(&mut host, 0x10);
    core.tick(&mut host).unwrap();

    let snapshot = core.snapshot();
    let player = snapshot
        .player
        .as_ref()
        .expect("R load should leave a live player");
    assert_eq!(
        snapshot.input_trace.jump_button_key, 0x10,
        "R load should preserve the sample's Shift jump binding; globals={:?}",
        core.globals
    );
    assert!(
        snapshot.input_trace.jump_just_pressed,
        "Shift should be observed as the jump edge after R load; trace={:?}",
        snapshot.input_trace
    );
    assert!(
        player.vspeed < 0.0,
        "Shift after R load should produce upward vspeed, got {}; player={:?}; players={:?}",
        player.vspeed,
        player,
        core.current_room()
            .unwrap()
            .instances
            .iter()
            .filter(|instance| crate::helpers::is_player_instance(instance))
            .map(|instance| (
                instance.runtime_id,
                instance.instance_id,
                instance.alive,
                instance.x,
                instance.y,
                instance.vspeed,
                instance.vars.clone()
            ))
            .collect::<Vec<_>>()
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

    let reset_key = 0x74;
    core.globals.insert(
        "global.restartbutton".into(),
        RuntimeValue::Number(reset_key as f64),
    );
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
fn real_sample_second_shift_press_after_manual_room_reload_uses_player_jump() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(step_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "object:0:event:3:0")
        {
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
                    target: LoweredLogicExpr::Identifier("debug_ground_branch_taken".into()),
                    value: LoweredLogicExpr::LiteralBool(false),
                },
            );
            if let Some(LoweredLogicStatement::Conditional { then_branch, .. }) =
                player_jump_entry.statements.get_mut(3)
            {
                if let Some(LoweredLogicStatement::Conditional {
                    then_branch: jump_ground_branch,
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
