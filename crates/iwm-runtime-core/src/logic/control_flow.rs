use iwm_runtime_host::RuntimeHost;

use super::context::{RuntimeEvalContext, RuntimeRoomInstanceOverlay};
use super::statement::RuntimeStatementEnvironment;
use crate::{LoweredLogicExpr, RuntimeInstance};

pub(super) fn env_has_pending_scene_change<H: RuntimeHost>(
    env: &RuntimeStatementEnvironment<'_, H>,
) -> bool {
    *env.pending_game_restart || *env.pending_room_reset || env.pending_room_transition.is_some()
}

pub(super) fn merged_statement_overlay<'a>(
    base_overlay: &RuntimeRoomInstanceOverlay<'a>,
    pending_updates: &[(usize, RuntimeInstance)],
    current_index: usize,
    current_instance: &RuntimeInstance,
) -> RuntimeRoomInstanceOverlay<'a> {
    base_overlay.merge_pending_current(pending_updates, current_index, current_instance)
}

pub(super) fn sync_instance_from_updates(
    current_index: usize,
    current_instance: &mut RuntimeInstance,
    pending_updates: &mut Vec<(usize, RuntimeInstance)>,
) {
    let Some(last_update_index) = pending_updates
        .iter()
        .rposition(|(index, _)| *index == current_index)
    else {
        return;
    };
    *current_instance = pending_updates[last_update_index].1.clone();
    pending_updates.retain(|(index, _)| *index != current_index);
}

pub(super) fn write_with_target_indices(
    target: &LoweredLogicExpr,
    instance_index: usize,
    context: &RuntimeEvalContext<'_>,
    output: &mut Vec<usize>,
) {
    output.clear();
    match target {
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("self") => {
            output.push(instance_index);
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
        }
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("all") => {
            output.extend(
                context
                    .room_instances_iter()
                    .filter(|(_, instance)| instance.alive)
                    .map(|(index, _)| index),
            );
        }
        LoweredLogicExpr::Identifier(name) => {
            let wanted_object_ids = context
                .place_target_ids_by_name
                .get(&name.to_ascii_lowercase())
                .map(Vec::as_slice)
                .unwrap_or(&[]);

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
        _ => {}
    }
}
