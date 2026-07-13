use std::collections::HashMap;
use std::path::Path;

use iwm_runtime_host::{
    Rgba8, RuntimeDrawCommand, RuntimeHost, RuntimeHostErrorKind, RuntimeSoundMode,
};
use iwm_runtime_model::{FontResource, ObjectDefinition, SpriteResource};

use super::assignment::{assign_runtime_value, next_room_id, runtime_value_to_room_id};
use super::calls::{
    dispatch_move_contact_solid, dispatch_runtime_sound_call, evaluate_file_bin_byte,
    evaluate_file_bin_handle, resolve_runtime_sound_id, runtime_value_to_i32,
};
use super::context::{
    RuntimeBinaryFileState, RuntimeEvalContext, RuntimeExecutionScope, RuntimeInstanceCreateRequest,
};
use super::control_flow::{
    env_has_pending_scene_change, merged_statement_overlay, sync_instance_from_updates,
    write_with_target_indices,
};
use super::diagnostics::{
    record_unsupported_expr_functions, record_unsupported_function, record_unsupported_statement,
    trace_message,
};
use super::eval::{assignable_key, is_truthy};
use super::eval_functions::{
    evaluate_collision_line_with_scratch, evaluate_distance_to_object_with_scratch,
    evaluate_instance_number_with_scratch,
};
use super::eval_variables::{evaluate_expr_with_sprite_constants, instance_member_access};
use super::instances::{
    assign_runtime_member_reference, pending_create_member_value,
    pending_create_member_value_by_object_target, runtime_instance_create_request,
};
use super::overlay::RuntimeSparseInstanceOverlay;
use crate::event_dispatch::{inherited_event_block_id, RuntimeEventSelector};
use crate::helpers::{as_number, record_host_diagnostic};
use crate::tick_context::RuntimeObjectQueryScratch;
use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicStatement, RuntimeInstance, RuntimeValue,
};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionTrace {
    pub(crate) room_id: usize,
    pub(crate) tick: u64,
    pub(crate) block_id: String,
    pub(crate) object_name: String,
    pub(crate) event_tag: String,
}

pub(crate) struct RuntimeStatementEnvironment<'a, H: RuntimeHost> {
    pub(crate) script_entries: &'a HashMap<String, LoweredLogicEntry>,
    pub(crate) sound_index: &'a HashMap<String, i32>,
    pub(crate) globals: &'a mut HashMap<String, RuntimeValue>,
    pub(crate) room_speed: &'a mut u32,
    pub(crate) pending_room_transition: &'a mut Option<usize>,
    pub(crate) pending_room_reset: &'a mut bool,
    pub(crate) pending_game_restart: &'a mut bool,
    pub(crate) binary_files: &'a mut RuntimeBinaryFileState,
    pub(crate) host: &'a mut H,
    pub(crate) diagnostics: &'a mut Vec<iwm_runtime_host::RuntimeDiagnostic>,
    pub(crate) object_query_scratch: Option<&'a mut RuntimeObjectQueryScratch>,
    pub(crate) with_target_indices: &'a mut Vec<usize>,
    pub(crate) room_instance_updates: &'a mut RuntimeSparseInstanceOverlay,
    pub(crate) room_instance_creates: &'a mut Vec<RuntimeInstanceCreateRequest>,
    pub(crate) objects: &'a [ObjectDefinition],
    pub(crate) sprites: &'a [SpriteResource],
    pub(crate) sprite_index: &'a HashMap<usize, usize>,
    pub(crate) sprite_ids_by_name: &'a HashMap<String, usize>,
    pub(crate) fonts: &'a [FontResource],
    pub(crate) font_index_by_name: &'a HashMap<String, usize>,
    pub(crate) lowered_entries: &'a [LoweredLogicEntry],
    pub(crate) event_selector: Option<RuntimeEventSelector>,
    pub(crate) event_owner_id: Option<usize>,
    pub(crate) draw: Option<&'a mut RuntimeDrawContext>,
    pub(crate) trace: RuntimeExecutionTrace,
}

#[derive(Debug)]
pub(crate) struct RuntimeDrawContext {
    colour: Rgba8,
    align: String,
    size: u32,
    font_name: Option<String>,
    font_bold: bool,
    font_italic: bool,
    commands: Vec<RuntimeDrawCommand>,
}

struct DrawFont {
    name: Option<String>,
    size: u32,
    bold: bool,
    italic: bool,
}

impl RuntimeDrawContext {
    pub(crate) fn finish(self) -> Vec<RuntimeDrawCommand> {
        self.commands
    }
}

impl Default for RuntimeDrawContext {
    fn default() -> Self {
        Self {
            colour: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            align: "left".into(),
            size: 16,
            font_name: None,
            font_bold: false,
            font_italic: false,
            commands: Vec::new(),
        }
    }
}

pub(crate) fn apply_runtime_statement<H: RuntimeHost>(
    statement: &LoweredLogicStatement,
    instance: &mut RuntimeInstance,
    instance_index: usize,
    scope: &mut RuntimeExecutionScope,
    destroy_event_entries: &HashMap<usize, Vec<LoweredLogicEntry>>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    match statement {
        LoweredLogicStatement::Assignment { target, value } => {
            if let Some(value) = evaluate_with_diagnostics(
                value,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            ) {
                if assign_runtime_member_reference(
                    target,
                    value.clone(),
                    instance,
                    instance_index,
                    scope,
                    eval_context,
                    env,
                ) {
                    return;
                }
                if let Some(key) = assignable_key(
                    target,
                    Some(instance),
                    env.globals,
                    Some(scope),
                    eval_context,
                ) {
                    assign_runtime_value(
                        key,
                        value,
                        instance,
                        env.globals,
                        scope,
                        Some(&mut *env.room_speed),
                        env.sprites,
                        env.sprite_index,
                    );
                }
            }
        }
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = evaluate_with_diagnostics(
                condition,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            );
            let branch = if is_truthy(condition_value) {
                then_branch
            } else {
                else_branch
            };
            for nested in branch {
                apply_runtime_statement(
                    nested,
                    instance,
                    instance_index,
                    scope,
                    destroy_event_entries,
                    eval_context,
                    env,
                );
            }
        }
        LoweredLogicStatement::VariableDeclaration { names } => {
            for name in names {
                scope.declare(name);
            }
        }
        LoweredLogicStatement::Return { .. } => {}
        LoweredLogicStatement::FunctionCall { name, args } => match name.as_str() {
            "room_goto" => {
                if let Some(room_id) = args
                    .first()
                    .and_then(|arg| {
                        evaluate_with_diagnostics(
                            arg,
                            Some(instance),
                            Some(scope),
                            eval_context,
                            env,
                            instance,
                        )
                    })
                    .and_then(|value| runtime_value_to_room_id(&value))
                    .filter(|room_id| {
                        eval_context
                            .map(|context| context.room_order.contains(room_id))
                            .unwrap_or(true)
                    })
                {
                    *env.pending_game_restart = false;
                    *env.pending_room_reset = false;
                    *env.pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-unresolved",
                        format!(
                            "could not resolve room_goto target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "room_goto_next" => {
                if let Some(room_id) = next_room_id(instance, env.globals, eval_context) {
                    *env.pending_game_restart = false;
                    *env.pending_room_reset = false;
                    *env.pending_room_transition = Some(room_id);
                } else {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-step-room-goto-next-unresolved",
                        format!(
                            "could not resolve room_goto_next target for {}",
                            instance.object_name
                        ),
                    );
                }
            }
            "game_restart" => {
                *env.pending_room_transition = None;
                *env.pending_room_reset = false;
                *env.pending_game_restart = true;
            }
            "draw_set_color" => {
                let Some(colour) = args.first().and_then(draw_colour_arg) else {
                    return;
                };
                if let Some(draw) = env.draw.as_deref_mut() {
                    draw.colour = colour;
                }
            }
            "draw_set_halign" => {
                let Some(align) = args.first().and_then(draw_align_arg) else {
                    return;
                };
                if let Some(draw) = env.draw.as_deref_mut() {
                    draw.align = align;
                }
            }
            "draw_set_font" => {
                let Some(font) = args.first().and_then(|arg| draw_font_arg(arg, env)) else {
                    return;
                };
                if let Some(draw) = env.draw.as_deref_mut() {
                    draw.size = font.size;
                    draw.font_name = font.name;
                    draw.font_bold = font.bold;
                    draw.font_italic = font.italic;
                }
            }
            "draw_text" => {
                dispatch_draw_text(args, instance, scope, eval_context, env);
            }
            "draw_sprite" => {
                dispatch_draw_sprite(args, instance, scope, eval_context, env);
            }
            "event_inherited" => {
                dispatch_event_inherited(
                    instance,
                    instance_index,
                    scope,
                    destroy_event_entries,
                    eval_context,
                    env,
                );
            }
            "sound_play" => {
                dispatch_runtime_sound_call(
                    env,
                    name,
                    args,
                    Some(RuntimeSoundMode::Once),
                    instance,
                    scope,
                    eval_context,
                );
            }
            "sound_loop" => {
                dispatch_runtime_sound_call(
                    env,
                    name,
                    args,
                    Some(RuntimeSoundMode::Loop),
                    instance,
                    scope,
                    eval_context,
                );
            }
            "sound_stop" => {
                dispatch_runtime_sound_call(env, name, args, None, instance, scope, eval_context);
            }
            "sound_stop_all" => {
                if let Err(error) = env.host.stop_all_sounds() {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-audio-host-error",
                        format!(
                            "{} function=sound_stop_all error={}",
                            trace_message(&env.trace, instance),
                            error
                        ),
                    );
                }
            }
            "keyboard_set_numlock" => {
                if let Some(value) = args.first().and_then(|arg| {
                    evaluate_with_diagnostics(
                        arg,
                        Some(instance),
                        Some(scope),
                        eval_context,
                        env,
                        instance,
                    )
                }) {
                    env.host.set_keyboard_numlock(is_truthy(Some(value)));
                }
            }
            "move_contact_solid" => {
                dispatch_move_contact_solid(env, args, instance, scope, eval_context);
            }
            "file_bin_write_byte" => {
                let Some(handle) =
                    evaluate_file_bin_handle(args.first(), instance, scope, eval_context, env)
                else {
                    return;
                };
                let Some(byte) =
                    evaluate_file_bin_byte(args.get(1), instance, scope, eval_context, env)
                else {
                    return;
                };
                env.binary_files.write_byte(handle, byte);
            }
            "file_delete" => {
                let Some(RuntimeValue::Text(path)) = args.first().and_then(|arg| {
                    evaluate_runtime_expr(
                        arg,
                        Some(instance),
                        Some(scope),
                        eval_context,
                        env,
                        instance,
                    )
                }) else {
                    return;
                };
                if let Err(error) = env.host.remove_temp(Path::new(&path)) {
                    if error.kind() != RuntimeHostErrorKind::NotFound {
                        record_host_diagnostic(
                            env.host,
                            env.diagnostics,
                            iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                            "runtime-file-host-error",
                            format!(
                                "{} function=file_delete path={} error={}",
                                trace_message(&env.trace, instance),
                                path,
                                error
                            ),
                        );
                    }
                }
            }
            "file_bin_close" => {
                let Some(handle) =
                    evaluate_file_bin_handle(args.first(), instance, scope, eval_context, env)
                else {
                    return;
                };
                if let Err(error) = env.binary_files.close(env.host, handle) {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-file-host-error",
                        format!(
                            "{} function=file_bin_close handle={} error={}",
                            trace_message(&env.trace, instance),
                            handle,
                            error
                        ),
                    );
                }
            }
            "__iwm_action_wrap" => {
                let Some(context) = eval_context else {
                    return;
                };
                let mode = args
                    .first()
                    .and_then(|arg| {
                        evaluate_with_diagnostics(
                            arg,
                            Some(instance),
                            Some(scope),
                            eval_context,
                            env,
                            instance,
                        )
                    })
                    .and_then(|value| as_number(&value))
                    .map(|value| value.round() as i32)
                    .unwrap_or(2);
                let image_xscale = instance
                    .vars
                    .get("image_xscale")
                    .and_then(as_number)
                    .unwrap_or(1.0);
                let image_yscale = instance
                    .vars
                    .get("image_yscale")
                    .and_then(as_number)
                    .unwrap_or(1.0);
                let sprite_width = instance.width as f64 * image_xscale;
                let sprite_height = instance.height as f64 * image_yscale;
                if mode != 1 {
                    let room_width = context.room_width as f64;
                    if instance.hspeed > 0.0 && instance.x > room_width {
                        instance.x -= room_width + sprite_width;
                    } else if instance.hspeed < 0.0 && instance.x < 0.0 {
                        instance.x += room_width + sprite_width;
                    }
                }
                if mode != 0 {
                    let room_height = context.room_height as f64;
                    if instance.vspeed > 0.0 && instance.y > room_height {
                        instance.y -= room_height + sprite_height;
                    } else if instance.vspeed < 0.0 && instance.y < 0.0 {
                        instance.y += room_height + sprite_height;
                    }
                }
            }
            "instance_destroy" => {
                if instance.alive {
                    let entries = destroy_event_entries
                        .get(&instance.object_id)
                        .cloned()
                        .unwrap_or_default();
                    for entry in &entries {
                        let mut destroy_scope = RuntimeExecutionScope::default();
                        let nested_destroy_entries = HashMap::new();
                        for nested in &entry.statements {
                            apply_runtime_statement(
                                nested,
                                instance,
                                instance_index,
                                &mut destroy_scope,
                                &nested_destroy_entries,
                                eval_context,
                                env,
                            );
                            if env_has_pending_scene_change(env) {
                                break;
                            }
                        }
                        if env_has_pending_scene_change(env) {
                            break;
                        }
                    }
                    instance.alive = false;
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                        "runtime-instance-destroyed",
                        format!(
                            "{} object={} runtime_id={}",
                            trace_message(&env.trace, instance),
                            instance.object_name,
                            instance.runtime_id
                        ),
                    );
                }
            }
            "instance_create" => {
                if let Some(create) = runtime_instance_create_request(
                    args,
                    instance,
                    env.globals,
                    scope,
                    eval_context,
                    env.room_instance_creates.len(),
                ) {
                    env.room_instance_creates.push(create);
                }
            }
            _ => {
                if let Some(entry) = env.script_entries.get(name) {
                    let mut script_scope = RuntimeExecutionScope::default();
                    let previous_trace = env.trace.clone();
                    env.trace.block_id.clone_from(&entry.block_id);
                    env.trace.event_tag = "script".into();
                    for nested in &entry.statements {
                        apply_runtime_statement(
                            nested,
                            instance,
                            instance_index,
                            &mut script_scope,
                            destroy_event_entries,
                            eval_context,
                            env,
                        );
                    }
                    env.trace = previous_trace;
                } else {
                    record_unsupported_function(env, name, instance);
                }
            }
        },
        LoweredLogicStatement::With { target, body } => {
            let Some(context) = eval_context else {
                return;
            };
            write_with_target_indices(
                target,
                instance_index,
                instance,
                scope,
                context,
                env.globals,
                env.with_target_indices,
            );
            let target_indices = std::mem::take(env.with_target_indices);
            let other_snapshot = instance.clone();
            for &target_index in &target_indices {
                if target_index == instance_index {
                    for nested in body {
                        let overlay = merged_statement_overlay(
                            &context.room_instance_overlay,
                            env.room_instance_updates,
                            instance_index,
                            instance,
                        );
                        let with_context = context.with_other_and_overlay(&other_snapshot, overlay);
                        apply_runtime_statement(
                            nested,
                            instance,
                            instance_index,
                            scope,
                            destroy_event_entries,
                            Some(&with_context),
                            env,
                        );
                        sync_instance_from_updates(
                            instance_index,
                            instance,
                            env.room_instance_updates,
                        );
                        if env_has_pending_scene_change(env) {
                            break;
                        }
                    }
                    if env_has_pending_scene_change(env) {
                        break;
                    }
                    continue;
                }

                let Some(mut target_instance) = context.room_instance(target_index).cloned() else {
                    continue;
                };
                sync_instance_from_updates(
                    target_index,
                    &mut target_instance,
                    env.room_instance_updates,
                );
                if !target_instance.alive {
                    continue;
                }
                for nested in body {
                    let overlay = merged_statement_overlay(
                        &context.room_instance_overlay,
                        env.room_instance_updates,
                        target_index,
                        &target_instance,
                    );
                    let with_context = context.with_other_and_overlay(&other_snapshot, overlay);
                    apply_runtime_statement(
                        nested,
                        &mut target_instance,
                        target_index,
                        scope,
                        destroy_event_entries,
                        Some(&with_context),
                        env,
                    );
                    sync_instance_from_updates(
                        target_index,
                        &mut target_instance,
                        env.room_instance_updates,
                    );
                    if env_has_pending_scene_change(env) {
                        break;
                    }
                }
                env.room_instance_updates.set(target_index, target_instance);
                if env_has_pending_scene_change(env) {
                    break;
                }
            }
            *env.with_target_indices = target_indices;
        }
        LoweredLogicStatement::For {
            init,
            condition,
            step,
            body,
        } => {
            execute_assignment_expression(init, instance, scope, eval_context, env);
            let mut iteration_count = 0usize;
            while is_truthy(evaluate_with_diagnostics(
                condition,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )) {
                for nested in body {
                    apply_runtime_statement(
                        nested,
                        instance,
                        instance_index,
                        scope,
                        destroy_event_entries,
                        eval_context,
                        env,
                    );
                    if env_has_pending_scene_change(env) {
                        break;
                    }
                }
                if env_has_pending_scene_change(env) {
                    break;
                }
                execute_assignment_expression(step, instance, scope, eval_context, env);
                iteration_count += 1;
                if iteration_count >= 10_000 {
                    record_host_diagnostic(
                        env.host,
                        env.diagnostics,
                        iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                        "runtime-for-iteration-limit",
                        format!(
                            "{} iteration_limit=10000",
                            trace_message(&env.trace, instance)
                        ),
                    );
                    break;
                }
            }
        }
        LoweredLogicStatement::Repeat { count, body } => {
            let repeat_count = evaluate_with_diagnostics(
                count,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
            .and_then(|value| as_number(&value))
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| value.floor() as usize)
            .unwrap_or(0);
            let capped_count = repeat_count.min(10_000);
            for _ in 0..capped_count {
                for nested in body {
                    apply_runtime_statement(
                        nested,
                        instance,
                        instance_index,
                        scope,
                        destroy_event_entries,
                        eval_context,
                        env,
                    );
                    if env_has_pending_scene_change(env) {
                        break;
                    }
                }
                if env_has_pending_scene_change(env) {
                    break;
                }
            }
            if repeat_count > capped_count {
                record_host_diagnostic(
                    env.host,
                    env.diagnostics,
                    iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                    "runtime-repeat-iteration-limit",
                    format!(
                        "{} iteration_limit=10000",
                        trace_message(&env.trace, instance)
                    ),
                );
            }
        }
        _ => {
            record_unsupported_statement(env, statement, instance);
        }
    }
}

fn dispatch_draw_text<H: RuntimeHost>(
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    scope: &mut RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    let Some(x) = args
        .first()
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
    else {
        return;
    };
    let Some(y) = args
        .get(1)
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
    else {
        return;
    };
    let Some(text) = args
        .get(2)
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .map(|value| runtime_value_to_string_text(&value))
    else {
        return;
    };
    if let Some(draw) = env.draw.as_deref_mut() {
        draw.commands.push(RuntimeDrawCommand::DrawText {
            text,
            x: x.round() as i32,
            y: y.round() as i32,
            size: draw.size,
            font_name: draw.font_name.clone(),
            font_bold: draw.font_bold,
            font_italic: draw.font_italic,
            colour: draw.colour,
            align: draw.align.clone(),
        });
    }
}

fn dispatch_draw_sprite<H: RuntimeHost>(
    args: &[LoweredLogicExpr],
    instance: &RuntimeInstance,
    scope: &mut RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    let Some(sprite_id) = args
        .first()
        .and_then(|arg| draw_sprite_id_arg(arg, instance, scope, eval_context, env))
    else {
        return;
    };
    let frame_index = args
        .get(1)
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.floor() as usize)
        .unwrap_or(0);
    let Some(x) = args
        .get(2)
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
    else {
        return;
    };
    let Some(y) = args
        .get(3)
        .and_then(|arg| {
            evaluate_runtime_expr(
                arg,
                Some(instance),
                Some(scope),
                eval_context,
                env,
                instance,
            )
        })
        .and_then(|value| as_number(&value))
    else {
        return;
    };
    let sprite = env
        .sprite_index
        .get(&sprite_id)
        .and_then(|index| env.sprites.get(*index));
    if let Some(draw) = env.draw.as_deref_mut() {
        draw.commands.push(RuntimeDrawCommand::DrawSprite {
            sprite_id,
            frame_index,
            x: x.round() as i32,
            y: y.round() as i32,
            origin_x: sprite.map(|sprite| sprite.origin_x).unwrap_or(0),
            origin_y: sprite.map(|sprite| sprite.origin_y).unwrap_or(0),
            xscale: 1.0,
            yscale: 1.0,
            alpha: 1.0,
            angle_degrees: 0.0,
        });
    }
}

fn draw_colour_arg(expr: &LoweredLogicExpr) -> Option<Rgba8> {
    let name = match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => {
            name.to_ascii_lowercase()
        }
        LoweredLogicExpr::LiteralNumber(number) if number.is_finite() => {
            return Some(gm_colour_number_to_rgba(*number as u32));
        }
        _ => return None,
    };
    Some(match name.as_str() {
        "c_black" => rgba(0, 0, 0),
        "c_white" => rgba(255, 255, 255),
        "c_red" => rgba(255, 0, 0),
        "c_green" => rgba(0, 128, 0),
        "c_blue" => rgba(0, 0, 255),
        "c_yellow" => rgba(255, 255, 0),
        "c_gray" | "c_grey" => rgba(128, 128, 128),
        "c_ltgray" | "c_ltgrey" => rgba(192, 192, 192),
        "c_dkgray" | "c_dkgrey" => rgba(64, 64, 64),
        _ => return None,
    })
}

fn draw_align_arg(expr: &LoweredLogicExpr) -> Option<String> {
    let name = match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => {
            name.to_ascii_lowercase()
        }
        _ => return None,
    };
    match name.as_str() {
        "fa_left" => Some("left".into()),
        "fa_center" => Some("center".into()),
        "fa_right" => Some("right".into()),
        _ => None,
    }
}

fn draw_font_arg<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    env: &RuntimeStatementEnvironment<'_, H>,
) -> Option<DrawFont> {
    if let LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) = expr {
        if let Some(font) = env
            .font_index_by_name
            .get(&name.to_ascii_lowercase())
            .and_then(|index| env.fonts.get(*index))
        {
            return Some(DrawFont {
                name: Some(font.name.clone()),
                size: font.size,
                bold: font.bold,
                italic: font.italic,
            });
        }
    }

    draw_font_size_arg(expr).map(|size| DrawFont {
        name: None,
        size,
        bold: false,
        italic: false,
    })
}

fn draw_font_size_arg(expr: &LoweredLogicExpr) -> Option<u32> {
    match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => name
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
            .filter(|size| *size > 0),
        LoweredLogicExpr::LiteralNumber(number) if number.is_finite() && *number > 0.0 => {
            Some(number.round() as u32)
        }
        _ => None,
    }
}

fn draw_sprite_id_arg<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: &RuntimeInstance,
    scope: &mut RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) -> Option<usize> {
    match expr {
        LoweredLogicExpr::Identifier(name) | LoweredLogicExpr::LiteralText(name) => env
            .sprite_ids_by_name
            .get(&name.to_ascii_lowercase())
            .copied(),
        LoweredLogicExpr::LiteralNumber(number) if number.is_finite() && *number >= 0.0 => {
            Some(number.round() as usize)
        }
        _ => evaluate_runtime_expr(
            expr,
            Some(instance),
            Some(scope),
            eval_context,
            env,
            instance,
        )
        .and_then(|value| as_number(&value))
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.round() as usize),
    }
}

fn gm_colour_number_to_rgba(value: u32) -> Rgba8 {
    rgba(
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
    )
}

fn rgba(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

fn dispatch_event_inherited<H: RuntimeHost>(
    instance: &mut RuntimeInstance,
    instance_index: usize,
    scope: &mut RuntimeExecutionScope,
    destroy_event_entries: &HashMap<usize, Vec<LoweredLogicEntry>>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    let Some(owner_object_id) = env.event_owner_id else {
        return;
    };
    let Some(selector) = env.event_selector.clone() else {
        return;
    };
    let Some((inherited_owner_id, block_id)) =
        inherited_event_block_id(env.objects, owner_object_id, &selector)
    else {
        return;
    };
    let Some(entry) = env
        .lowered_entries
        .iter()
        .find(|entry| entry.block_id == block_id)
        .cloned()
    else {
        return;
    };

    let previous_owner_id = env.event_owner_id;
    let previous_trace = env.trace.clone();
    env.event_owner_id = Some(inherited_owner_id);
    env.trace.block_id.clone_from(&entry.block_id);

    for nested in &entry.statements {
        apply_runtime_statement(
            nested,
            instance,
            instance_index,
            scope,
            destroy_event_entries,
            eval_context,
            env,
        );
    }

    env.trace = previous_trace;
    env.event_owner_id = previous_owner_id;
}

fn execute_assignment_expression<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: &mut RuntimeInstance,
    scope: &mut RuntimeExecutionScope,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
) {
    if let LoweredLogicExpr::BinaryExpr { op, left, right } = expr {
        if op == "=" {
            if let Some(key) =
                assignable_key(left, Some(instance), env.globals, Some(scope), eval_context)
            {
                if let Some(value) = evaluate_with_diagnostics(
                    right,
                    Some(instance),
                    Some(scope),
                    eval_context,
                    env,
                    instance,
                ) {
                    assign_runtime_value(
                        key,
                        value,
                        instance,
                        env.globals,
                        scope,
                        Some(&mut *env.room_speed),
                        env.sprites,
                        env.sprite_index,
                    );
                }
            }
        }
    }
}

pub(super) fn evaluate_with_diagnostics<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    let value = evaluate_runtime_expr(expr, instance, scope, eval_context, env, trace_instance);
    if value.is_none() {
        record_unsupported_expr_functions(env, expr, trace_instance);
    }
    value
}

fn evaluate_runtime_expr<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    if let LoweredLogicExpr::Identifier(name) = expr {
        if name.eq_ignore_ascii_case("room_speed")
            && !scope.map(|scope| scope.is_local_key(name)).unwrap_or(false)
        {
            return Some(RuntimeValue::Number(*env.room_speed as f64));
        }
    }
    if let LoweredLogicExpr::UnaryExpr { op, child } = expr {
        let value =
            evaluate_runtime_expr(child, instance, scope, eval_context, env, trace_instance)?;
        return match op.as_str() {
            "-" => Some(RuntimeValue::Number(-as_number(&value)?)),
            "+" => Some(RuntimeValue::Number(as_number(&value)?)),
            "!" => Some(RuntimeValue::Bool(!is_truthy(Some(value)))),
            _ => None,
        };
    }
    if let LoweredLogicExpr::BinaryExpr { op, left, right } = expr {
        let left = evaluate_runtime_binary_operand(
            left,
            instance,
            scope,
            eval_context,
            env,
            trace_instance,
        )?;
        let right = evaluate_runtime_binary_operand(
            right,
            instance,
            scope,
            eval_context,
            env,
            trace_instance,
        )?;
        return eval_runtime_binary_expr(op, &left, &right);
    }
    if let LoweredLogicExpr::Call { name, args } = expr {
        match name.as_str() {
            "instance_number" => {
                if let Some(scratch) = env.object_query_scratch.as_deref_mut() {
                    return evaluate_instance_number_with_scratch(args, eval_context, scratch);
                }
            }
            "distance_to_object" => {
                if let Some(scratch) = env.object_query_scratch.as_deref_mut() {
                    return evaluate_distance_to_object_with_scratch(
                        args,
                        instance,
                        eval_context,
                        scratch,
                    );
                }
            }
            "collision_line" => {
                if let Some(scratch) = env.object_query_scratch.as_deref_mut() {
                    return evaluate_collision_line_with_scratch(
                        args,
                        instance,
                        env.globals,
                        scope,
                        eval_context,
                        scratch,
                    );
                }
            }
            _ => {}
        }
        if name == "instance_create" {
            let instance = instance?;
            let scope = scope?;
            return runtime_instance_create_request(
                args,
                instance,
                env.globals,
                scope,
                eval_context,
                env.room_instance_creates.len(),
            )
            .map(|create| {
                let instance_id = create.instance_id;
                env.room_instance_creates.push(create);
                RuntimeValue::Number(instance_id as f64)
            });
        }
        if name == "sound_isplaying" {
            let sound_id = args.first().and_then(|arg| {
                resolve_runtime_sound_id(
                    arg,
                    trace_instance,
                    scope,
                    eval_context,
                    env.globals,
                    env.sound_index,
                )
            })?;
            return env
                .host
                .is_sound_playing(sound_id)
                .ok()
                .map(RuntimeValue::Bool);
        }
        if name == "keyboard_get_numlock" {
            return Some(RuntimeValue::Bool(env.host.keyboard_numlock()));
        }
        if name == "file_exists" {
            let path = args.first().and_then(|arg| {
                evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
            })?;
            let RuntimeValue::Text(path) = path else {
                return None;
            };
            return Some(RuntimeValue::Bool(env.host.read(Path::new(&path)).is_ok()));
        }
        if name == "file_bin_open" {
            let path = args.first().and_then(|arg| {
                evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
            })?;
            let RuntimeValue::Text(path) = path else {
                return None;
            };
            let mode = args
                .get(1)
                .and_then(|arg| {
                    evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
                })
                .and_then(|value| as_number(&value))
                .map(|value| value.round() as i32)
                .unwrap_or(0);
            let handle = env.binary_files.open(&*env.host, path, mode);
            return Some(RuntimeValue::Number(handle as f64));
        }
        if name == "file_bin_read_byte" {
            let handle = args
                .first()
                .and_then(|arg| {
                    evaluate_runtime_expr(arg, instance, scope, eval_context, env, trace_instance)
                })
                .and_then(|value| runtime_value_to_i32(&value))?;
            let byte = env.binary_files.read_byte(handle);
            return Some(RuntimeValue::Number(byte as f64));
        }
    }
    if let Some((target, member)) =
        instance_member_access(expr, instance, env.globals, scope, eval_context)
    {
        if let Some(value) = pending_create_member_value_by_object_target(
            env.room_instance_creates,
            target,
            &member,
            scope,
            eval_context,
        ) {
            return Some(value);
        }
        if let Some(RuntimeValue::Number(instance_ref)) =
            evaluate_runtime_expr(target, instance, scope, eval_context, env, trace_instance)
        {
            if let Some(value) =
                pending_create_member_value(env.room_instance_creates, instance_ref, &member)
            {
                return Some(value);
            }
        }
    }

    evaluate_expr_with_sprite_constants(
        expr,
        instance,
        env.globals,
        scope,
        eval_context,
        env.sprite_ids_by_name,
    )
}

fn evaluate_runtime_binary_operand<H: RuntimeHost>(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
    eval_context: Option<&RuntimeEvalContext<'_>>,
    env: &mut RuntimeStatementEnvironment<'_, H>,
    trace_instance: &RuntimeInstance,
) -> Option<RuntimeValue> {
    evaluate_runtime_expr(expr, instance, scope, eval_context, env, trace_instance)
        .or_else(|| uninitialized_runtime_instance_operand(expr, instance, scope))
}

fn uninitialized_runtime_instance_operand(
    expr: &LoweredLogicExpr,
    instance: Option<&RuntimeInstance>,
    scope: Option<&RuntimeExecutionScope>,
) -> Option<RuntimeValue> {
    let LoweredLogicExpr::Identifier(name) = expr else {
        return None;
    };
    if instance.is_some()
        && !name.eq_ignore_ascii_case("global")
        && !scope.map(|scope| scope.is_local_key(name)).unwrap_or(false)
    {
        Some(RuntimeValue::Number(0.0))
    } else {
        None
    }
}

fn eval_runtime_binary_expr(
    op: &str,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> Option<RuntimeValue> {
    match op {
        "+" => match (left, right) {
            (RuntimeValue::Text(_), _) | (_, RuntimeValue::Text(_)) => {
                Some(RuntimeValue::Text(format!(
                    "{}{}",
                    runtime_value_to_string_text(left),
                    runtime_value_to_string_text(right)
                )))
            }
            _ => Some(RuntimeValue::Number(as_number(left)? + as_number(right)?)),
        },
        "-" => Some(RuntimeValue::Number(as_number(left)? - as_number(right)?)),
        "*" => Some(RuntimeValue::Number(as_number(left)? * as_number(right)?)),
        "/" => Some(RuntimeValue::Number(as_number(left)? / as_number(right)?)),
        "div" => {
            let divisor = as_number(right)?;
            if divisor == 0.0 {
                return None;
            }
            Some(RuntimeValue::Number((as_number(left)? / divisor).trunc()))
        }
        "mod" => {
            let divisor = as_number(right)?;
            if divisor == 0.0 {
                return None;
            }
            Some(RuntimeValue::Number(as_number(left)? % divisor))
        }
        "==" | "=" => Some(RuntimeValue::Bool(runtime_values_equal(left, right))),
        "!=" => Some(RuntimeValue::Bool(!runtime_values_equal(left, right))),
        ">=" => Some(RuntimeValue::Bool(as_number(left)? >= as_number(right)?)),
        "<=" => Some(RuntimeValue::Bool(as_number(left)? <= as_number(right)?)),
        ">" => Some(RuntimeValue::Bool(as_number(left)? > as_number(right)?)),
        "<" => Some(RuntimeValue::Bool(as_number(left)? < as_number(right)?)),
        "&&" => Some(RuntimeValue::Bool(
            is_truthy(Some(left.clone())) && is_truthy(Some(right.clone())),
        )),
        "||" => Some(RuntimeValue::Bool(
            is_truthy(Some(left.clone())) || is_truthy(Some(right.clone())),
        )),
        _ => None,
    }
}

fn runtime_value_to_string_text(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Number(number) if number.fract() == 0.0 => format!("{}", *number as i64),
        RuntimeValue::Number(number) => number.to_string(),
        RuntimeValue::Bool(flag) => flag.to_string(),
        RuntimeValue::Text(text) => text.clone(),
    }
}

fn runtime_values_equal(left: &RuntimeValue, right: &RuntimeValue) -> bool {
    match (as_number(left), as_number(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}
