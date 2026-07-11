use iwm_runtime_model::ObjectEventEntry;

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{add_step_block, append_lowered_entry, host, sample_package};

fn player_animation_value(core: &RuntimeCore, key: &str) -> f64 {
    core.current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap()
        .vars
        .get(key)
        .and_then(|value| match value {
            RuntimeValue::Number(number) => Some(*number),
            _ => None,
        })
        .unwrap()
}

fn sprite_switch_core(target_frame_count: usize) -> RuntimeCore {
    let mut package = sample_package();
    package.resources.sprites[0].frame_paths = (0..4)
        .map(|frame| format!("resources/sprites/0-{frame}.png"))
        .collect();
    package.resources.sprites[1].name = "spr_target".into();
    package.resources.sprites[1].frame_paths = (0..target_frame_count)
        .map(|frame| format!("resources/sprites/1-{frame}.png"))
        .collect();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("sprite_index".into()),
            value: LoweredLogicExpr::Identifier("spr_target".into()),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let player = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.player_candidate)
        .unwrap();
    player
        .vars
        .insert("image_index".into(), RuntimeValue::Number(2.5));
    player
        .vars
        .insert("image_speed".into(), RuntimeValue::Number(0.5));

    let mut host = host();
    core.tick(&mut host).unwrap();
    core
}

#[test]
fn runtime_core_sprite_switch_preserves_valid_fractional_image_index() {
    let core = sprite_switch_core(4);

    assert_eq!(player_animation_value(&core, "sprite_index"), 1.0);
    assert_eq!(player_animation_value(&core, "image_index"), 3.0);
    assert_eq!(player_animation_value(&core, "image_speed"), 0.5);
}

#[test]
fn runtime_core_sprite_switch_resets_out_of_range_image_index_before_advancing() {
    let core = sprite_switch_core(2);

    assert_eq!(player_animation_value(&core, "sprite_index"), 1.0);
    assert_eq!(player_animation_value(&core, "image_index"), 0.5);
    assert_eq!(player_animation_value(&core, "image_speed"), 0.5);
}

#[test]
fn runtime_core_advances_image_index_from_image_speed_each_tick() {
    let mut package = sample_package();
    package.resources.sprites[1].frame_paths = vec![
        "resources/sprites/1-0.png".into(),
        "resources/sprites/1-1.png".into(),
        "resources/sprites/1-2.png".into(),
    ];
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker
        .vars
        .insert("sprite_index".into(), crate::RuntimeValue::Number(1.0));
    marker
        .vars
        .insert("image_index".into(), crate::RuntimeValue::Number(0.0));
    marker
        .vars
        .insert("image_speed".into(), crate::RuntimeValue::Number(0.5));

    core.tick(&mut host).unwrap();

    let marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    let image_index = marker
        .vars
        .get("image_index")
        .and_then(|value| match value {
            crate::RuntimeValue::Number(number) => Some(*number),
            _ => None,
        })
        .unwrap();
    assert_eq!(image_index, 0.5);
}

#[test]
fn runtime_core_wraps_image_index_by_sprite_frame_count() {
    let mut package = sample_package();
    package.resources.sprites[1].frame_paths = vec![
        "resources/sprites/1-0.png".into(),
        "resources/sprites/1-1.png".into(),
        "resources/sprites/1-2.png".into(),
    ];
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker
        .vars
        .insert("sprite_index".into(), crate::RuntimeValue::Number(1.0));
    marker
        .vars
        .insert("image_index".into(), crate::RuntimeValue::Number(2.5));
    marker
        .vars
        .insert("image_speed".into(), crate::RuntimeValue::Number(1.0));

    core.tick(&mut host).unwrap();

    let marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    let image_index = marker
        .vars
        .get("image_index")
        .and_then(|value| match value {
            crate::RuntimeValue::Number(number) => Some(*number),
            _ => None,
        })
        .unwrap();
    assert_eq!(image_index, 0.5);
}

#[test]
fn runtime_core_dispatches_animation_end_when_image_index_wraps() {
    let mut package = sample_package();
    package.resources.sprites[1].frame_paths = vec![
        "resources/sprites/1-0.png".into(),
        "resources/sprites/1-1.png".into(),
        "resources/sprites/1-2.png".into(),
    ];
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 7,
        sub_event: 7,
        event_tag: "other:animation-end".into(),
        block_id: "object:1:event:7:7".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:7:7".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("animation_done".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker
        .vars
        .insert("sprite_index".into(), RuntimeValue::Number(1.0));
    marker
        .vars
        .insert("image_index".into(), RuntimeValue::Number(2.5));
    marker
        .vars
        .insert("image_speed".into(), RuntimeValue::Number(1.0));

    core.tick(&mut host).unwrap();

    let marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert_eq!(
        marker.vars.get("animation_done"),
        Some(&RuntimeValue::Bool(true))
    );
}
