use iwm_runtime_host::RuntimeDrawCommand;

use crate::RuntimeCore;

use super::support::{host, sample_package};

#[test]
fn runtime_core_emits_browser_consumable_draw_commands() {
    let package = sample_package();
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.room_id, Some(7));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawBackground { background_id: 0, .. }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawTile {
            background_id: 0,
            width: 32,
            height: 32,
            ..
        }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite { sprite_id: 0, .. }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite { sprite_id: 1, .. }
    )));
}
