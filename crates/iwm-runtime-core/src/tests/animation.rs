use iwm_runtime_model::ObjectEventEntry;

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{append_lowered_entry, host, sample_package};

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
