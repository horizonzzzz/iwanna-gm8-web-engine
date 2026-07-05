use super::*;

#[test]
fn core_reports_unsupported_statement_with_execution_context() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::While {
            condition: LoweredLogicExpr::LiteralBool(false),
            body: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("x".into()),
                value: LoweredLogicExpr::LiteralNumber(99.0),
            }],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let diagnostics = core.diagnostics();
    let unsupported = diagnostics
        .iter()
        .find(|entry| entry.code == "runtime-unsupported-statement")
        .expect("unsupported while statement should be diagnosed");
    assert!(unsupported.message.contains("room=7"));
    assert!(unsupported.message.contains("tick=1"));
    assert!(unsupported.message.contains("block_id=object:0:event:3:0"));
    assert!(unsupported.message.contains("object=obj_player"));
    assert!(unsupported.message.contains("event_tag=step"));
    assert!(unsupported.message.contains("statement_kind=while"));
    assert!(unsupported.message.contains("runtime_id=0"));
}

#[test]
fn core_reports_unsupported_function_with_execution_context() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_position".into(),
            args: vec![
                LoweredLogicExpr::Identifier("x".into()),
                LoweredLogicExpr::Identifier("y".into()),
                LoweredLogicExpr::Identifier("obj_marker".into()),
            ],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let diagnostics = core.diagnostics();
    let unsupported = diagnostics
        .iter()
        .find(|entry| entry.code == "runtime-unsupported-function")
        .expect("unsupported function should be diagnosed");
    assert!(unsupported.message.contains("room=7"));
    assert!(unsupported.message.contains("tick=1"));
    assert!(unsupported.message.contains("block_id=object:0:event:3:0"));
    assert!(unsupported.message.contains("object=obj_player"));
    assert!(unsupported.message.contains("event_tag=step"));
    assert!(unsupported.message.contains("function=instance_position"));
    assert!(unsupported.message.contains("runtime_id=0"));
}

#[test]
fn core_reports_unsupported_expression_function_with_execution_context() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "instance_position".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::Identifier("y".into()),
                    LoweredLogicExpr::Identifier("obj_marker".into()),
                ],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("x".into()),
                value: LoweredLogicExpr::LiteralNumber(99.0),
            }],
            else_branch: vec![],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let diagnostics = core.diagnostics();
    let unsupported = diagnostics
        .iter()
        .find(|entry| entry.code == "runtime-unsupported-function")
        .expect("unsupported expression function should be diagnosed");
    assert!(unsupported.message.contains("room=7"));
    assert!(unsupported.message.contains("tick=1"));
    assert!(unsupported.message.contains("block_id=object:0:event:3:0"));
    assert!(unsupported.message.contains("object=obj_player"));
    assert!(unsupported.message.contains("event_tag=step"));
    assert!(unsupported.message.contains("function=instance_position"));
    assert!(unsupported.message.contains("runtime_id=0"));
}

#[test]
fn core_keeps_block_level_execution_trace_out_of_normal_ticks() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("hspeed".into()),
            value: LoweredLogicExpr::LiteralNumber(1.0),
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let diagnostics = core.diagnostics();
    assert!(diagnostics
        .iter()
        .all(|entry| entry.code != "runtime-exec-block-trace"));
}

#[test]
fn core_does_not_report_supported_abs_or_string_functions() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("speed_text".into()),
                value: LoweredLogicExpr::Call {
                    name: "string".into(),
                    args: vec![LoweredLogicExpr::Call {
                        name: "abs".into(),
                        args: vec![LoweredLogicExpr::LiteralNumber(-4.0)],
                    }],
                },
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "instance_position".into(),
                    args: vec![
                        LoweredLogicExpr::Identifier("x".into()),
                        LoweredLogicExpr::Identifier("y".into()),
                        LoweredLogicExpr::Identifier("obj_marker".into()),
                    ],
                },
                then_branch: vec![],
                else_branch: vec![],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    let unsupported_functions = core
        .diagnostics()
        .iter()
        .filter(|entry| entry.code == "runtime-unsupported-function")
        .map(|entry| entry.message.as_str())
        .collect::<Vec<_>>();
    assert!(unsupported_functions
        .iter()
        .all(|message| !message.contains("function=abs")));
    assert!(unsupported_functions
        .iter()
        .all(|message| !message.contains("function=string")));
    assert!(unsupported_functions
        .iter()
        .any(|message| message.contains("function=instance_position")));
}

#[test]
fn core_does_not_report_supported_keyboard_get_numlock() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "keyboard_get_numlock".into(),
                args: vec![],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("numlock_seen".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    let unsupported_functions = core
        .diagnostics()
        .iter()
        .filter(|entry| entry.code == "runtime-unsupported-function")
        .map(|entry| entry.message.as_str())
        .collect::<Vec<_>>();
    assert!(unsupported_functions
        .iter()
        .all(|message| !message.contains("function=keyboard_get_numlock")));
}
