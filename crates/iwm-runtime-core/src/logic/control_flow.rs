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

pub(super) fn with_target_indices(
    target: &LoweredLogicExpr,
    instance_index: usize,
    context: &RuntimeEvalContext<'_>,
) -> Vec<usize> {
    match target {
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("self") => {
            vec![instance_index]
        }
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("other") => context
            .other_instance()
            .and_then(|other| {
                context
                    .room_instances_iter()
                    .find(|(_, instance)| instance.runtime_id == other.runtime_id)
                    .map(|(index, _)| index)
            })
            .into_iter()
            .collect(),
        LoweredLogicExpr::Identifier(name) if name.eq_ignore_ascii_case("all") => context
            .room_instances_iter()
            .filter(|(_, instance)| instance.alive)
            .map(|(index, _)| index)
            .collect(),
        LoweredLogicExpr::Identifier(name) => {
            let wanted_object_ids = context
                .place_target_ids_by_name
                .get(&name.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default();
            context
                .room_instances_matching_object_ids(&wanted_object_ids)
                .filter(|(_, instance)| instance.alive)
                .map(|(index, _)| index)
                .collect()
        }
        _ => Vec::new(),
    }
}
