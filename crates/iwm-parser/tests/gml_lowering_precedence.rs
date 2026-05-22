use iwm_parser::gml_lowering::lower_raw_logic_file;
use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};
use iwm_parser::models::{RawLogicFile, RawLogicScript};

#[test]
fn lowering_respects_boolean_precedence_between_or_and_and() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 1,
            script_name: "scr_logic".to_string(),
            gml_source: "flag = a || b && c;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "flag"));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "||"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "a")
                    && matches!(
                        right.as_ref(),
                        LoweredLogicExpr::BinaryExpr { op, left, right }
                            if op == "&&"
                            && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "b")
                            && matches!(right.as_ref(), LoweredLogicExpr::Identifier(name) if name == "c")
                    )
            ));
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}
