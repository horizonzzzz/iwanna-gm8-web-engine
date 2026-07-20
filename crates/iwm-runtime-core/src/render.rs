use std::collections::HashMap;

use iwm_runtime_host::RuntimeHost;
use iwm_runtime_host::{RuntimeDrawCommand, RuntimeRenderFrame};
use iwm_runtime_model::RuntimeDisplaySource;

use crate::event_dispatch::{
    event_owner_id_for_block_id, object_event_block_ids,
    runtime_instance_indices_by_object_id_from_instances, RuntimeEventSelector,
};
use crate::helpers::as_number;
use crate::logic::{
    apply_runtime_statement, commit_instance_updates, gm_colour_number_to_rgba,
    sync_current_instance_from_updates, RuntimeDrawContext, RuntimeExecutionScope,
    RuntimeRoomInstanceOverlay, RuntimeSparseInstanceOverlay, RuntimeStatementEnvironment,
};
use crate::{RuntimeCore, RuntimeCoreError, RuntimeInstance, RuntimeRoomState};

#[derive(Debug, Clone, Copy)]
struct ActiveView {
    source_x: i32,
    source_y: i32,
    source_w: u32,
    source_h: u32,
    port_x: i32,
    port_y: i32,
    port_w: u32,
    port_h: u32,
}

impl ActiveView {
    fn source_bounds(self) -> (i32, i32, i32, i32) {
        (
            self.source_x,
            self.source_y,
            self.source_x + self.source_w as i32,
            self.source_y + self.source_h as i32,
        )
    }

    fn frame_width(self) -> u32 {
        (self.port_x.max(0) as u32 + self.port_w).max(1)
    }

    fn frame_height(self) -> u32 {
        (self.port_y.max(0) as u32 + self.port_h).max(1)
    }

    fn translate_x(self, x: i32) -> i32 {
        x - self.source_x + self.port_x
    }

    fn translate_y(self, y: i32) -> i32 {
        y - self.source_y + self.port_y
    }
}

fn active_view_for_room(room: &RuntimeRoomState) -> Option<ActiveView> {
    if !room.views_enabled {
        return None;
    }

    room.views
        .iter()
        .find(|view| view.visible)
        .and_then(|view| {
            if view.source_w == 0 || view.source_h == 0 || view.port_w == 0 || view.port_h == 0 {
                None
            } else {
                Some(ActiveView {
                    source_x: view.source_x,
                    source_y: view.source_y,
                    source_w: view.source_w,
                    source_h: view.source_h,
                    port_x: view.port_x,
                    port_y: view.port_y,
                    port_w: view.port_w,
                    port_h: view.port_h,
                })
            }
        })
}

impl RuntimeCore {
    pub(crate) fn build_render_frame(
        &self,
        draw_commands: Vec<RuntimeDrawCommand>,
    ) -> Result<RuntimeRenderFrame, RuntimeCoreError> {
        let room = self
            .current_room
            .as_ref()
            .ok_or(RuntimeCoreError::NoRooms)?;
        let source_room = self
            .room_index
            .get(&room.room_id)
            .and_then(|index| self.package.rooms.get(*index))
            .ok_or(RuntimeCoreError::RoomMissing(room.room_id))?;
        let active_view = active_view_for_room(room);
        let active_bounds = active_view.map(ActiveView::source_bounds);
        let manifest_display = match (
            self.package.manifest.display_source,
            self.package.manifest.display_width,
            self.package.manifest.display_height,
        ) {
            (
                Some(RuntimeDisplaySource::ExeResolution),
                Some(display_width),
                Some(display_height),
            ) => Some((display_width, display_height)),
            _ => None,
        };
        let frame_width = active_view
            .map(ActiveView::frame_width)
            .or_else(|| manifest_display.map(|(width, _)| width))
            .unwrap_or(room.width);
        let frame_height = active_view
            .map(ActiveView::frame_height)
            .or_else(|| manifest_display.map(|(_, height)| height))
            .unwrap_or(room.height);

        // Pre-size for clear + backgrounds + tiles + per-instance sprites +
        // present, so large rooms don't repeatedly grow the command buffer.
        let estimated_commands =
            8 + source_room.backgrounds.len() + source_room.tiles.len() + room.instances.len();
        let mut commands = Vec::with_capacity(estimated_commands);
        if source_room.clear_screen {
            commands.push(RuntimeDrawCommand::Clear {
                colour: gm_colour_number_to_rgba(source_room.background_colour),
            });
        }

        commands.extend(
            source_room
                .backgrounds
                .iter()
                .filter(|layer| {
                    layer.visible_on_start && !layer.is_foreground && layer.source_bg >= 0
                })
                .map(|layer| RuntimeDrawCommand::DrawBackground {
                    background_id: layer.source_bg as usize,
                    x: active_view
                        .map(|view| view.translate_x(layer.xoffset))
                        .unwrap_or(layer.xoffset),
                    y: active_view
                        .map(|view| view.translate_y(layer.yoffset))
                        .unwrap_or(layer.yoffset),
                    stretch: layer.stretch,
                    tile_horz: layer.tile_horz,
                    tile_vert: layer.tile_vert,
                    is_foreground: false,
                }),
        );

        let mut depth_commands = source_room
            .tiles
            .iter()
            .filter(|tile| tile.source_bg >= 0)
            .filter(|tile| {
                active_bounds.is_none_or(|(left, top, right, bottom)| {
                    rect_intersects_view(
                        tile.x,
                        tile.y,
                        tile.x + tile.width as i32,
                        tile.y + tile.height as i32,
                        left,
                        top,
                        right,
                        bottom,
                    )
                })
            })
            .map(|tile| {
                (
                    tile.depth,
                    RuntimeDrawCommand::DrawTile {
                        background_id: tile.source_bg as usize,
                        x: active_view
                            .map(|view| view.translate_x(tile.x))
                            .unwrap_or(tile.x),
                        y: active_view
                            .map(|view| view.translate_y(tile.y))
                            .unwrap_or(tile.y),
                        tile_x: tile.tile_x,
                        tile_y: tile.tile_y,
                        width: tile.width,
                        height: tile.height,
                        xscale: tile.xscale,
                        yscale: tile.yscale,
                    },
                )
            })
            .collect::<Vec<_>>();

        for instance in &room.instances {
            if !instance.alive {
                continue;
            }
            let Some(object) = self
                .object_index
                .get(&instance.object_id)
                .and_then(|index| self.package.objects.get(*index))
            else {
                continue;
            };

            if !runtime_instance_visible(instance) {
                continue;
            }

            let sprite_id = runtime_instance_sprite_id(instance, object.sprite_index);
            if sprite_id >= 0 {
                let sprite = self
                    .sprite_index
                    .get(&(sprite_id as usize))
                    .and_then(|index| self.package.resources.sprites.get(*index));
                let sprite_width = sprite.map(|sprite| sprite.width as i32).unwrap_or(16);
                let sprite_height = sprite.map(|sprite| sprite.height as i32).unwrap_or(16);
                if let Some((left, top, right, bottom)) = active_bounds {
                    if !rect_intersects_view(
                        instance.x.round() as i32,
                        instance.y.round() as i32,
                        instance.x.round() as i32 + sprite_width,
                        instance.y.round() as i32 + sprite_height,
                        left,
                        top,
                        right,
                        bottom,
                    ) {
                        continue;
                    }
                }
                depth_commands.push((
                    runtime_instance_depth(instance, &self.package.objects, &self.object_index),
                    RuntimeDrawCommand::DrawSprite {
                        sprite_id: sprite_id as usize,
                        frame_index: runtime_instance_frame_index(instance),
                        x: active_view
                            .map(|view| view.translate_x(instance.x.round() as i32))
                            .unwrap_or(instance.x.round() as i32),
                        y: active_view
                            .map(|view| view.translate_y(instance.y.round() as i32))
                            .unwrap_or(instance.y.round() as i32),
                        origin_x: sprite.map(|sprite| sprite.origin_x).unwrap_or(0),
                        origin_y: sprite.map(|sprite| sprite.origin_y).unwrap_or(0),
                        xscale: runtime_instance_xscale(instance),
                        yscale: runtime_instance_yscale(instance),
                        alpha: runtime_instance_alpha(instance),
                        angle_degrees: 0.0,
                    },
                ));
            }
        }

        depth_commands.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
        commands.extend(depth_commands.into_iter().map(|(_, command)| command));

        commands.extend(draw_commands);

        commands.extend(
            source_room
                .backgrounds
                .iter()
                .filter(|layer| {
                    layer.visible_on_start && layer.is_foreground && layer.source_bg >= 0
                })
                .map(|layer| RuntimeDrawCommand::DrawBackground {
                    background_id: layer.source_bg as usize,
                    x: active_view
                        .map(|view| view.translate_x(layer.xoffset))
                        .unwrap_or(layer.xoffset),
                    y: active_view
                        .map(|view| view.translate_y(layer.yoffset))
                        .unwrap_or(layer.yoffset),
                    stretch: layer.stretch,
                    tile_horz: layer.tile_horz,
                    tile_vert: layer.tile_vert,
                    is_foreground: true,
                }),
        );

        commands.push(RuntimeDrawCommand::Present);

        Ok(RuntimeRenderFrame {
            tick: self.tick,
            room_id: Some(room.room_id),
            width: frame_width,
            height: frame_height,
            commands,
        })
    }

    pub(crate) fn execute_draw_events<H: RuntimeHost>(
        &mut self,
        host: &mut H,
    ) -> Result<Vec<RuntimeDrawCommand>, RuntimeCoreError> {
        let script_entries = &self.cached_script_entries;
        let destroy_event_entries = &self.cached_destroy_event_entries;
        let objects = &self.package.objects;
        let lowered_entries = self
            .package
            .lowered_logic
            .as_ref()
            .map(|logic| logic.entries.as_slice())
            .unwrap_or(&[]);
        let room_order = &self.cached_room_order;
        let button_states = host.active_buttons().into_iter().collect::<HashMap<_, _>>();
        let (
            current_room_id,
            mut current_room_speed,
            dispatches,
            room_instance_indices_by_object_id,
        ) = {
            let Some(room) = self.current_room.as_ref() else {
                return Err(RuntimeCoreError::NoRooms);
            };
            let dispatches = room
                .instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| instance.alive)
                .filter_map(|(index, instance)| {
                    let entries = object_event_block_ids(
                        &self.package,
                        instance.object_id,
                        RuntimeEventSelector::Draw,
                    )
                    .iter()
                    .filter_map(|block_id| self.lowered_logic_entry(block_id).cloned())
                    .collect::<Vec<_>>();
                    (!entries.is_empty()).then_some((index, entries))
                })
                .collect::<Vec<_>>();
            (
                room.room_id,
                room.speed,
                dispatches,
                runtime_instance_indices_by_object_id_from_instances(&room.instances),
            )
        };

        let mut draw_context = RuntimeDrawContext::default();
        let mut instance_updates = RuntimeSparseInstanceOverlay::default();
        let mut instance_creates = Vec::new();

        for (index, entries) in dispatches {
            let Some(mut instance) = instance_updates.get(index).cloned().or_else(|| {
                self.current_room
                    .as_ref()
                    .and_then(|room| room.instances.get(index).cloned())
            }) else {
                continue;
            };
            if !instance.alive {
                continue;
            }

            for entry in &entries {
                let event_owner_id = event_owner_id_for_block_id(objects, &entry.block_id)
                    .unwrap_or(instance.object_id);
                let mut scope = RuntimeExecutionScope::default();
                let mut with_updates = RuntimeSparseInstanceOverlay::default();
                for statement in &entry.statements {
                    let Some(room) = self.current_room.as_ref() else {
                        return Err(RuntimeCoreError::NoRooms);
                    };
                    let eval_overlay = RuntimeRoomInstanceOverlay::with_current(
                        &instance_updates,
                        &with_updates,
                        index,
                        &instance,
                    );
                    let eval_context = crate::logic::RuntimeEvalContext {
                        current_room_id,
                        room_speed: current_room_speed,
                        room_width: room.width,
                        room_height: room.height,
                        random_state: &self.random_state,
                        button_states: &button_states,
                        room_instances: &room.instances,
                        room_instance_indices_by_object_id: &room_instance_indices_by_object_id,
                        object_index: None,
                        collision_spatial_index: None,
                        room_instance_overlay: eval_overlay,
                        room_order: room_order.as_slice(),
                        other_instance: None,
                        other_runtime_id: None,
                        place_target_ids_by_name: &self.place_target_ids_by_name,
                        room_ids_by_name: &self.room_ids_by_name,
                        view_zero: crate::logic::RuntimeViewValues::from_room(room),
                    };
                    let mut with_target_indices = Vec::new();
                    let mut statement_env = RuntimeStatementEnvironment {
                        script_entries,
                        sound_index: &self.sound_index,
                        globals: &mut self.globals,
                        room_speed: &mut current_room_speed,
                        pending_room_transition: &mut self.pending_room_transition,
                        pending_room_reset: &mut self.pending_room_reset,
                        pending_game_restart: &mut self.pending_game_restart,
                        binary_files: &mut self.binary_files,
                        host: &mut *host,
                        diagnostics: &mut self.diagnostics,
                        object_query_scratch: None,
                        with_target_indices: &mut with_target_indices,
                        room_instance_updates: &mut with_updates,
                        room_instance_creates: &mut instance_creates,
                        objects,
                        sprites: &self.package.resources.sprites,
                        paths: &self.package.resources.paths,
                        sprite_index: &self.sprite_index,
                        sprite_ids_by_name: &self.sprite_ids_by_name,
                        fonts: &self.package.resources.fonts,
                        font_index_by_name: &self.font_index_by_name,
                        zero_uninitialized_vars: self.package.manifest.zero_uninitialized_vars,
                        lowered_entries,
                        event_selector: Some(RuntimeEventSelector::Draw),
                        event_owner_id: Some(event_owner_id),
                        draw: Some(&mut draw_context),
                        trace: crate::logic::RuntimeExecutionTrace {
                            room_id: current_room_id,
                            tick: self.tick,
                            block_id: entry.block_id.clone(),
                            object_name: instance.object_name.clone(),
                            event_tag: "draw".into(),
                        },
                    };
                    apply_runtime_statement(
                        statement,
                        &mut instance,
                        index,
                        &mut scope,
                        &destroy_event_entries,
                        Some(&eval_context),
                        &mut statement_env,
                    );
                    sync_current_instance_from_updates(index, &mut instance, &mut with_updates);
                    if self.has_pending_scene_change() {
                        break;
                    }
                }
                commit_instance_updates(&mut instance_updates, &mut with_updates);
                if self.has_pending_scene_change() {
                    break;
                }
            }
            if self.has_pending_scene_change() {
                break;
            }
        }

        if let Some(room) = self.current_room.as_mut() {
            room.speed = current_room_speed;
            for (index, updated_instance) in instance_updates.drain_dirty_updates() {
                if let Some(slot) = room.instances.get_mut(index) {
                    *slot = updated_instance;
                }
            }
        }
        self.apply_runtime_instance_creates(host, &mut instance_creates);

        Ok(draw_context.finish())
    }
}

fn runtime_instance_visible(instance: &RuntimeInstance) -> bool {
    instance
        .vars
        .get("visible")
        .map(|value| match value {
            crate::RuntimeValue::Bool(flag) => *flag,
            crate::RuntimeValue::Number(number) => *number >= 0.5,
            crate::RuntimeValue::Text(text) => !text.is_empty(),
        })
        .unwrap_or(instance.visible)
}

fn runtime_instance_sprite_id(instance: &RuntimeInstance, object_sprite_index: i32) -> i32 {
    instance
        .vars
        .get("sprite_index")
        .and_then(as_number)
        .map(|value| value.round() as i32)
        .unwrap_or(object_sprite_index)
}

fn runtime_instance_frame_index(instance: &RuntimeInstance) -> usize {
    instance
        .vars
        .get("image_index")
        .and_then(as_number)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.floor() as usize)
        .unwrap_or(0)
}

fn runtime_instance_xscale(instance: &RuntimeInstance) -> f64 {
    let xscale = instance
        .vars
        .get("image_xscale")
        .and_then(as_number)
        .unwrap_or(1.0);
    if instance.facing_left {
        -xscale.abs()
    } else {
        xscale
    }
}

fn runtime_instance_yscale(instance: &RuntimeInstance) -> f64 {
    instance
        .vars
        .get("image_yscale")
        .and_then(as_number)
        .unwrap_or(1.0)
}

fn runtime_instance_alpha(instance: &RuntimeInstance) -> f64 {
    instance
        .vars
        .get("image_alpha")
        .and_then(as_number)
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

fn runtime_instance_depth(
    instance: &RuntimeInstance,
    objects: &[iwm_runtime_model::ObjectDefinition],
    object_index: &HashMap<usize, usize>,
) -> i32 {
    instance
        .vars
        .get("depth")
        .and_then(as_number)
        .filter(|value| value.is_finite())
        .map(|value| value.round() as i32)
        .or_else(|| {
            object_index
                .get(&instance.object_id)
                .and_then(|index| objects.get(*index))
                .map(|object| object.depth)
        })
        .unwrap_or(0)
}

fn rect_intersects_view(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    view_left: i32,
    view_top: i32,
    view_right: i32,
    view_bottom: i32,
) -> bool {
    left < view_right && right > view_left && top < view_bottom && bottom > view_top
}
