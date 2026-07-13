use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;

use super::context::{RuntimeEvalContext, RuntimeExecutionScope, RuntimeRoomInstanceOverlay};
use super::eval::evaluate_expr;
use super::overlay::RuntimeSparseInstanceOverlay;
use super::statement::RuntimeStatementEnvironment;
use crate::helpers::as_number;
use crate::{LoweredLogicExpr, RuntimeInstance, RuntimeValue};

pub(super) fn env_has_pending_scene_change<H: RuntimeHost>(
    env: &RuntimeStatementEnvironment<'_, H>,
) -> bool {
    *env.pending_game_restart || *env.pending_room_reset || env.pending_room_transition.is_some()
}

pub(super) fn merged_statement_overlay<'a>(
    base_overlay: &RuntimeRoomInstanceOverlay<'a>,
    pending_updates: &RuntimeSparseInstanceOverlay,
    current_index: usize,
    current_instance: &RuntimeInstance,
) -> RuntimeRoomInstanceOverlay<'a> {
    base_overlay.merge_pending_current(pending_updates, current_index, current_instance)
}

pub(super) fn sync_instance_from_updates(
    current_index: usize,
    current_instance: &mut RuntimeInstance,
    pending_updates: &mut RuntimeSparseInstanceOverlay,
) {
    if let Some(instance) = pending_updates.take(current_index) {
        *current_instance = instance;
    }
}

pub(super) fn write_with_target_indices(
    target: &LoweredLogicExpr,
    instance_index: usize,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    context: &RuntimeEvalContext<'_>,
    globals: &HashMap<String, RuntimeValue>,
    output: &mut Vec<usize>,
) {
    output.clear();
    match target {
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("self") => {
            output.push(instance_index);
            return;
        }
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("other") => {
            if let Some(index) = context.other_instance().and_then(|other| {
                context
                    .room_instances_iter()
                    .find(|(_, instance)| instance.runtime_id == other.runtime_id)
                    .map(|(index, _)| index)
            }) {
                output.push(index);
            }
            return;
        }
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("all") => {
            output.extend(
                context
                    .room_instances_iter()
                    .filter(|(_, instance)| instance.alive)
                    .map(|(index, _)| index),
            );
            return;
        }
        LoweredLogicExpr::Call { name, args } if name == "__iwm_object" => {
            let Some(object_id) = args.first().and_then(|arg| match arg {
                LoweredLogicExpr::LiteralNumber(value) if value.is_finite() && *value >= 0.0 => {
                    Some(value.round() as usize)
                }
                _ => None,
            }) else {
                return;
            };
            output.extend(
                context
                    .room_instances_iter()
                    .filter(|(_, candidate)| candidate.alive && candidate.object_id == object_id)
                    .map(|(index, _)| index),
            );
        }
        LoweredLogicExpr::Identifier(name) if scope.is_local_key(name) => {
            push_with_instance_ref_target(target, instance, scope, context, globals, output);
        }
        LoweredLogicExpr::Identifier(name) => {
            let wanted_object_ids = context
                .place_target_ids_by_name
                .get(&name.to_ascii_lowercase())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if wanted_object_ids.is_empty() {
                push_with_instance_ref_target(target, instance, scope, context, globals, output);
                return;
            }

            if let Some(object_index) = context.object_index {
                for object_id in wanted_object_ids {
                    for &index in object_index.indices_for_object_id(*object_id) {
                        if context
                            .room_instance(index)
                            .is_some_and(|instance| instance.alive)
                        {
                            output.push(index);
                        }
                    }
                }
                return;
            }

            if context.room_instance_indices_by_object_id.is_empty() {
                output.extend(
                    context
                        .room_instances_iter()
                        .filter_map(|(index, instance)| {
                            (instance.alive && wanted_object_ids.contains(&instance.object_id))
                                .then_some(index)
                        }),
                );
                return;
            }

            for object_id in wanted_object_ids {
                if let Some(indices) = context.room_instance_indices_by_object_id.get(object_id) {
                    output.extend(indices.iter().copied().filter(|&index| {
                        context
                            .room_instance(index)
                            .is_some_and(|instance| instance.alive)
                    }));
                }
            }
        }
        _ => push_with_instance_ref_target(target, instance, scope, context, globals, output),
    }
}

fn push_with_instance_ref_target(
    target: &LoweredLogicExpr,
    instance: &RuntimeInstance,
    scope: &RuntimeExecutionScope,
    context: &RuntimeEvalContext<'_>,
    globals: &HashMap<String, RuntimeValue>,
    output: &mut Vec<usize>,
) {
    let Some(instance_ref) =
        evaluate_expr(target, Some(instance), globals, Some(scope), Some(context))
            .and_then(|value| as_number(&value))
            .filter(|value| value.is_finite())
            .map(|value| value.round())
    else {
        return;
    };

    output.extend(
        context
            .room_instances_iter()
            .filter_map(|(index, candidate)| {
                (candidate.alive
                    && (candidate.instance_id as f64 == instance_ref
                        || candidate.runtime_id as f64 == instance_ref))
                    .then_some(index)
            }),
    );
}
