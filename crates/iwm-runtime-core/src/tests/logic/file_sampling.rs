use super::*;

#[test]
fn core_samples_known_files_once_for_all_step_dispatches_in_a_tick() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("player_step_ran".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 3,
        sub_event: 0,
        event_tag: "step".into(),
        block_id: "object:1:event:3:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:3:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("marker_step_ran".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = ReadCountingHost::new();

    core.tick(&mut host).unwrap();

    assert_eq!(
        host.read_count.get(),
        5,
        "known file probing should be shared across all step owners for one tick"
    );
}
