use super::*;

#[test]
fn core_applies_room_create_script_calls_to_globals_for_control_bootstrap() {
    let mut package = sample_package();
    add_room_create_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "defControls".into(),
            args: vec![],
        }],
    );
    add_script_block(
        &mut package,
        16,
        "defControls",
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "jumpbutton".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0x10 as f64),
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.jumpbutton"),
        Some(&RuntimeValue::Number(0x10 as f64))
    );
}

#[test]
fn create_logic_instance_create_bootstraps_world_globals_immediately() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "world".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: true,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![ObjectEventEntry {
            event_type: 0,
            sub_event: 0,
            event_tag: "create".into(),
            block_id: "object:4:event:0:0".into(),
            action_count: 0,
        }],
    });
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Call {
                    name: "instance_exists".into(),
                    args: vec![LoweredLogicExpr::Identifier("world".into())],
                }),
                right: Box::new(LoweredLogicExpr::LiteralBool(false)),
            },
            then_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_create".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::Identifier("world".into()),
                ],
            }],
            else_branch: vec![],
        }],
    );
    append_lowered_entry(
        &mut package,
        "object:4:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "grav".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0.0),
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.grav"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert!(core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .any(|instance| instance.object_name == "world"));
}
