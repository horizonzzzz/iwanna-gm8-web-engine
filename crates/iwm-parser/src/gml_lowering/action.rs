use iwm_runtime_model::{LoweredLogicExpr, LoweredLogicStatement};

use crate::models::RawCodeAction;

use super::expression::lower_expr;
use super::source::{looks_like_gml_source, lower_source};

pub(super) fn lower_action_list(actions: &[RawCodeAction]) -> Vec<LoweredLogicStatement> {
    let mut cursor = 0;
    lower_sequence(actions, &mut cursor, false)
}

fn lower_sequence(
    actions: &[RawCodeAction],
    cursor: &mut usize,
    stop_at_end: bool,
) -> Vec<LoweredLogicStatement> {
    let mut statements = Vec::new();
    while let Some(action) = actions.get(*cursor) {
        match action.action_kind {
            2 if stop_at_end => {
                *cursor += 1;
                break;
            }
            3 => break,
            _ => statements.extend(lower_next(actions, cursor)),
        }
    }
    statements
}

fn lower_next(actions: &[RawCodeAction], cursor: &mut usize) -> Vec<LoweredLogicStatement> {
    let Some(action) = actions.get(*cursor) else {
        return Vec::new();
    };

    if action.is_condition {
        let condition = lower_condition(action);
        *cursor += 1;
        let then_branch = lower_next(actions, cursor);
        let else_branch = if actions
            .get(*cursor)
            .is_some_and(|action| action.action_kind == 3)
        {
            *cursor += 1;
            lower_next(actions, cursor)
        } else {
            Vec::new()
        };
        return vec![LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        }];
    }

    match action.action_kind {
        1 => {
            *cursor += 1;
            lower_sequence(actions, cursor, true)
        }
        2 | 3 => {
            *cursor += 1;
            Vec::new()
        }
        4 => {
            *cursor += 1;
            vec![LoweredLogicStatement::Return { value: None }]
        }
        5 => {
            let count = action
                .args
                .first()
                .map(|value| lower_expr(value))
                .unwrap_or(LoweredLogicExpr::LiteralNumber(0.0));
            *cursor += 1;
            let body = lower_next(actions, cursor);
            vec![LoweredLogicStatement::Repeat { count, body }]
        }
        6 => {
            *cursor += 1;
            lower_variable_action(action)
        }
        _ => {
            *cursor += 1;
            wrap_applies_to(action, lower_action_source(action))
        }
    }
}

fn lower_condition(action: &RawCodeAction) -> LoweredLogicExpr {
    let condition = match action.fn_name.as_str() {
        "action_if_dice" => {
            let bound = action.args.first().map(String::as_str).unwrap_or("0");
            lower_expr(&format!("random({bound}) < 1"))
        }
        "action_if_number" => {
            let object = action.args.first().map(String::as_str).unwrap_or("-4");
            let number = action.args.get(1).map(String::as_str).unwrap_or("0");
            let comparator = action.args.get(2).map(String::as_str).unwrap_or("0");
            let operator = match comparator.trim() {
                "1" => "<",
                "2" => ">",
                _ => "==",
            };
            lower_expr(&format!("instance_number({object}) {operator} {number}"))
        }
        "action_if_variable" => {
            let variable = action.args.first().map(String::as_str).unwrap_or("0");
            let value = action.args.get(1).map(String::as_str).unwrap_or("0");
            let comparator = action.args.get(2).map(String::as_str).unwrap_or("0");
            let operator = match comparator.trim() {
                "1" => "<",
                "2" => ">",
                _ => "==",
            };
            lower_expr(&format!("{variable} {operator} {value}"))
        }
        _ if !action.fn_code.trim().is_empty() => lower_expr(action.fn_code.trim()),
        _ if !action.fn_name.is_empty() => {
            lower_expr(&format!("{}({})", action.fn_name, action.args.join(", ")))
        }
        _ => action
            .args
            .first()
            .map(|value| lower_expr(value))
            .unwrap_or(LoweredLogicExpr::LiteralBool(false)),
    };

    if action.invert_condition {
        LoweredLogicExpr::UnaryExpr {
            op: "!".into(),
            child: Box::new(condition),
        }
    } else {
        condition
    }
}

fn lower_variable_action(action: &RawCodeAction) -> Vec<LoweredLogicStatement> {
    let Some(name) = action.args.first() else {
        return Vec::new();
    };
    let value = action.args.get(1).map(String::as_str).unwrap_or("0");
    let source = if action.is_relative {
        format!("{name} += {value};")
    } else {
        format!("{name} = {value};")
    };
    wrap_applies_to(action, lower_source(&source))
}

fn lower_action_source(action: &RawCodeAction) -> Vec<LoweredLogicStatement> {
    let primary = action.fn_code.trim();
    if !primary.is_empty() {
        return lower_source(primary);
    }

    if let Some(source) = lower_function_action_source(action) {
        return lower_source(&source);
    }

    action
        .args
        .iter()
        .filter(|arg| looks_like_gml_source(arg))
        .flat_map(|arg| lower_source(arg))
        .collect()
}

fn lower_function_action_source(action: &RawCodeAction) -> Option<String> {
    let relative = |value: &str, axis: &str| {
        if action.is_relative {
            format!("{axis} + ({value})")
        } else {
            value.to_string()
        }
    };

    match action.fn_name.as_str() {
        "action_set_alarm" => {
            let time = action.args.first()?;
            let alarm = action.args.get(1)?;
            Some(format!("alarm[{alarm}] = {time};"))
        }
        "action_create_object" => {
            let object_id = action.args.first()?;
            let x = relative(action.args.get(1)?, "x");
            let y = relative(action.args.get(2)?, "y");
            Some(format!("instance_create({x}, {y}, {object_id});"))
        }
        "action_create_object_motion" => {
            let object_id = action.args.first()?;
            let x = relative(action.args.get(1)?, "x");
            let y = relative(action.args.get(2)?, "y");
            let speed = action.args.get(3)?;
            let direction = action.args.get(4)?;
            Some(format!(
                "var __iwm_action_created; __iwm_action_created = instance_create({x}, {y}, {object_id}); __iwm_action_created.speed = {speed}; __iwm_action_created.direction = {direction};"
            ))
        }
        "action_kill_object" => Some("instance_destroy();".into()),
        "action_set_motion" => {
            let direction = action.args.first()?;
            let speed = action.args.get(1)?;
            let direction = if action.is_relative {
                format!("direction + ({direction})")
            } else {
                direction.clone()
            };
            let speed = if action.is_relative {
                format!("speed + ({speed})")
            } else {
                speed.clone()
            };
            Some(format!("direction = {direction}; speed = {speed};"))
        }
        "action_timeline_set" => {
            let index = action.args.first()?;
            let position = action.args.get(1)?;
            let running = action.args.get(2)?.trim() == "0";
            let looping = action.args.get(3)?.trim() == "1";
            Some(format!(
                "timeline_index = {index}; timeline_position = {position}; timeline_running = {running}; timeline_loop = {looping};"
            ))
        }
        "action_timeline_start" => Some("timeline_running = true;".into()),
        "action_timeline_pause" => Some("timeline_running = false;".into()),
        "action_timeline_stop" => Some("timeline_position = 0; timeline_running = false;".into()),
        "action_set_timeline_position" => {
            let value = action.args.first()?;
            Some(if action.is_relative {
                format!("timeline_position += {value};")
            } else {
                format!("timeline_position = {value};")
            })
        }
        "action_set_timeline_speed" => {
            let value = action.args.first()?;
            Some(if action.is_relative {
                format!("timeline_speed += {value};")
            } else {
                format!("timeline_speed = {value};")
            })
        }
        "action_sprite_set" => {
            let sprite = action.args.first()?;
            let image_index = action.args.get(1)?;
            let image_speed = action.args.get(2)?;
            Some(format!(
                "sprite_index = {sprite}; image_index = {image_index}; image_speed = {image_speed};"
            ))
        }
        "action_sprite_transform" => {
            let mut xscale = action.args.first()?.clone();
            let mut yscale = action.args.get(1)?.clone();
            let angle = action.args.get(2)?;
            match action.args.get(3)?.trim() {
                "1" => xscale = format!("-({xscale})"),
                "2" => yscale = format!("-({yscale})"),
                "3" => {
                    xscale = format!("-({xscale})");
                    yscale = format!("-({yscale})");
                }
                _ => {}
            }
            Some(format!(
                "image_xscale = {xscale}; image_yscale = {yscale}; image_angle = {angle};"
            ))
        }
        "action_sound" => {
            let sound = action.args.first()?;
            let function = if action.args.get(1)?.trim() == "0" {
                "sound_play"
            } else {
                "sound_loop"
            };
            Some(format!("{function}({sound});"))
        }
        "action_wrap" => Some(format!(
            "__iwm_action_wrap({});",
            action.args.first().map(String::as_str).unwrap_or("2")
        )),
        _ => None,
    }
}

fn wrap_applies_to(
    action: &RawCodeAction,
    statements: Vec<LoweredLogicStatement>,
) -> Vec<LoweredLogicStatement> {
    if statements.is_empty() || action.applies_to == -1 {
        return statements;
    }

    let target = match action.applies_to {
        -2 => LoweredLogicExpr::Identifier("other".into()),
        -3 => LoweredLogicExpr::Identifier("all".into()),
        object_id if object_id >= 0 => LoweredLogicExpr::Call {
            name: "__iwm_object".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(object_id as f64)],
        },
        _ => return statements,
    };
    vec![LoweredLogicStatement::With {
        target,
        body: statements,
    }]
}
