use iwm_runtime_host::{Rgba8, RuntimeDrawCommand, RuntimeRenderFrame};

use crate::{RuntimeCore, RuntimeCoreError};

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
        let active_view = source_room
            .views_enabled
            .then(|| source_room.views.iter().find(|view| view.visible))
            .flatten()
            .map(|view| {
                (
                    view.source_x,
                    view.source_y,
                    view.source_x + view.source_w as i32,
                    view.source_y + view.source_h as i32,
                )
            });

        let mut commands = vec![RuntimeDrawCommand::Clear {
            colour: Rgba8 {
                r: 12,
                g: 16,
                b: 22,
                a: 255,
            },
        }];

        commands.extend(
            source_room
                .backgrounds
                .iter()
                .filter(|layer| {
                    layer.visible_on_start && !layer.is_foreground && layer.source_bg >= 0
                })
                .map(|layer| RuntimeDrawCommand::DrawBackground {
                    background_id: layer.source_bg as usize,
                    x: layer.xoffset,
                    y: layer.yoffset,
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
                    active_view.is_none_or(|(left, top, right, bottom)| {
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
                    x: tile.x,
                    y: tile.y,
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
                    .package
                    .resources
                    .sprites
                    .iter()
                    .find(|sprite| sprite.id == object.sprite_index as usize);
                let sprite_width = sprite.map(|sprite| sprite.width as i32).unwrap_or(16);
                let sprite_height = sprite.map(|sprite| sprite.height as i32).unwrap_or(16);
                if let Some((left, top, right, bottom)) = active_view {
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
                let sprite = self
                    .package
                    .resources
                    .sprites
                    .iter()
                    .find(|sprite| sprite.id == object.sprite_index as usize);
                commands.push(RuntimeDrawCommand::DrawSprite {
                    sprite_id: object.sprite_index as usize,
                    frame_index: 0,
                    x: instance.x.round() as i32,
                    y: instance.y.round() as i32,
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
                    x: layer.xoffset,
                    y: layer.yoffset,
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
            width: room.width,
            height: room.height,
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
