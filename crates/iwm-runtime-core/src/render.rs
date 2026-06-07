use iwm_runtime_host::{Rgba8, RuntimeDrawCommand, RuntimeRenderFrame};

use crate::{RuntimeCore, RuntimeCoreError, RuntimeRoomState};

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
    pub(crate) fn build_render_frame(&self) -> Result<RuntimeRenderFrame, RuntimeCoreError> {
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
        let frame_width = active_view
            .map(ActiveView::frame_width)
            .unwrap_or(room.width);
        let frame_height = active_view
            .map(ActiveView::frame_height)
            .unwrap_or(room.height);

        // Pre-size for clear + backgrounds + tiles + per-instance sprites +
        // present, so large rooms don't repeatedly grow the command buffer.
        let estimated_commands =
            2 + source_room.backgrounds.len() + source_room.tiles.len() + room.instances.len();
        let mut commands = Vec::with_capacity(estimated_commands);
        commands.push(RuntimeDrawCommand::Clear {
            colour: Rgba8 {
                r: 12,
                g: 16,
                b: 22,
                a: 255,
            },
        });

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

        commands.extend(
            source_room
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
                .map(|tile| RuntimeDrawCommand::DrawTile {
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
                }),
        );

        for instance in &room.instances {
            let Some(object) = self
                .object_index
                .get(&instance.object_id)
                .and_then(|index| self.package.objects.get(*index))
            else {
                continue;
            };

            if !object.visible {
                continue;
            }

            if object.sprite_index >= 0 {
                let sprite = self
                    .sprite_index
                    .get(&(object.sprite_index as usize))
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
                commands.push(RuntimeDrawCommand::DrawSprite {
                    sprite_id: object.sprite_index as usize,
                    frame_index: 0,
                    x: active_view
                        .map(|view| view.translate_x(instance.x.round() as i32))
                        .unwrap_or(instance.x.round() as i32),
                    y: active_view
                        .map(|view| view.translate_y(instance.y.round() as i32))
                        .unwrap_or(instance.y.round() as i32),
                    origin_x: sprite.map(|sprite| sprite.origin_x).unwrap_or(0),
                    origin_y: sprite.map(|sprite| sprite.origin_y).unwrap_or(0),
                    xscale: if instance.facing_left { -1.0 } else { 1.0 },
                    yscale: 1.0,
                    angle_degrees: 0.0,
                });
            }
        }

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
