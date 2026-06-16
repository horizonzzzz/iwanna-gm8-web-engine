use crate::RuntimeCore;

use super::support::{host, sample_package};

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
