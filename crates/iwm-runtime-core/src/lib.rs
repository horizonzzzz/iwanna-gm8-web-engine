use std::collections::HashMap;

use iwm_runtime_model::{
    AnalysisReport, ObjectDefinition, ResourceIndex, RoomDefinition, RuntimeManifest, ScriptIrFile,
};
use iwm_runtime_host::{
    Rgba8, RuntimeButton, RuntimeDrawCommand, RuntimeHost, RuntimeHostError, RuntimeRenderFrame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePackage {
    pub manifest: RuntimeManifest,
    pub rooms: Vec<RoomDefinition>,
    pub objects: Vec<ObjectDefinition>,
    pub scripts: ScriptIrFile,
    pub resources: ResourceIndex,
    pub analysis: AnalysisReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatus {
    Idle,
    Ready,
    Running,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuntimeInstance {
    pub runtime_id: usize,
    pub instance_id: i32,
    pub object_id: usize,
    pub object_name: String,
    pub x: i32,
    pub y: i32,
    pub previous_x: i32,
    pub previous_y: i32,
    pub hspeed: i32,
    pub vspeed: i32,
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub alive: bool,
    pub solid: bool,
    pub hazard: bool,
    pub checkpoint: bool,
    pub player_candidate: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeRoomState {
    pub room_id: usize,
    pub room_name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub playable: bool,
    pub transition_targets: Vec<usize>,
    pub spawn_point: Option<(i32, i32)>,
    pub instances: Vec<RuntimeInstance>,
}

#[derive(Debug, Clone)]
pub struct RuntimePlayerSnapshot {
    pub x: i32,
    pub y: i32,
    pub hspeed: i32,
    pub vspeed: i32,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub instance_count: usize,
    pub player: Option<RuntimePlayerSnapshot>,
    pub diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
}

#[derive(Debug)]
pub enum RuntimeCoreError {
    NoRooms,
    RoomMissing(usize),
    Host(RuntimeHostError),
}

impl From<RuntimeHostError> for RuntimeCoreError {
    fn from(value: RuntimeHostError) -> Self {
        Self::Host(value)
    }
}

#[derive(Debug)]
pub struct RuntimeCore {
    package: RuntimePackage,
    room_index: HashMap<usize, usize>,
    current_room: Option<RuntimeRoomState>,
    status: RuntimeStatus,
    tick: u64,
    diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
    pending_room_transition: Option<usize>,
    pending_room_reset: bool,
}

const RUN_SPEED: i32 = 4;
const JUMP_SPEED: i32 = 8;
const GRAVITY: i32 = 1;
const MAX_FALL_SPEED: i32 = 8;

impl RuntimeCore {
    pub fn load(package: RuntimePackage) -> Result<Self, RuntimeCoreError> {
        if package.rooms.is_empty() {
            return Err(RuntimeCoreError::NoRooms);
        }

        let room_index = package
            .rooms
            .iter()
            .enumerate()
            .map(|(index, room)| (room.id, index))
            .collect::<HashMap<_, _>>();

        let mut core = Self {
            package,
            room_index,
            current_room: None,
            status: RuntimeStatus::Ready,
            tick: 0,
            diagnostics: Vec::new(),
            pending_room_transition: None,
            pending_room_reset: false,
        };

        core.boot_default_room()?;
        Ok(core)
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
    }

    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    pub fn current_room(&self) -> Option<&RuntimeRoomState> {
        self.current_room.as_ref()
    }

    pub fn diagnostics(&self) -> &[iwm_runtime_host::RuntimeDiagnostic] {
        &self.diagnostics
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            status: self.status,
            tick: self.tick,
            room_id: self.current_room.as_ref().map(|room| room.room_id),
            room_name: self
                .current_room
                .as_ref()
                .map(|room| room.room_name.clone()),
            instance_count: self
                .current_room
                .as_ref()
                .map(|room| room.instances.len())
                .unwrap_or(0),
            player: self.current_room.as_ref().and_then(|room| {
                room.instances
                    .iter()
                    .find(|instance| is_player_instance(instance))
                    .map(|instance| RuntimePlayerSnapshot {
                        x: instance.x,
                        y: instance.y,
                        hspeed: instance.hspeed,
                        vspeed: instance.vspeed,
                    })
            }),
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn boot_default_room(&mut self) -> Result<(), RuntimeCoreError> {
        let room_id = self
            .package
            .manifest
            .default_room_id
            .or_else(|| self.package.rooms.first().map(|room| room.id))
            .ok_or(RuntimeCoreError::NoRooms)?;

        self.current_room = Some(self.build_room(room_id)?);
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    pub fn request_room_transition(&mut self, room_id: usize) {
        self.pending_room_transition = Some(room_id);
    }

    pub fn render<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        let frame = self.build_render_frame()?;
        host.submit_frame(frame)?;
        Ok(())
    }

    pub fn tick<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        if self.current_room.is_none() {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        }

        let left = host.button_state(RuntimeButton::Keyboard(0x25));
        let right = host.button_state(RuntimeButton::Keyboard(0x27));
        let jump = host.button_state(RuntimeButton::Keyboard(0x20));
        let restart = host.button_state(RuntimeButton::Keyboard(0x52));

        self.tick += 1;
        self.status = RuntimeStatus::Running;

        if !left.pressed && !right.pressed && !jump.pressed && !restart.pressed {
            self.push_diagnostic(
                iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                "runtime-idle",
                format!("tick {} advanced without player input", self.tick),
            );
        }

        if restart.just_pressed || restart.pressed {
            self.pending_room_reset = true;
            self.apply_pending_room_change()?;
            self.render(host)?;
            return Ok(());
        }

        self.apply_pending_room_change()?;

        let Some(room) = self.current_room.as_ref() else {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        };

        if room.instances.is_empty() {
            self.diagnostics.push(iwm_runtime_host::RuntimeDiagnostic {
                level: iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                code: "runtime-empty-room".into(),
                message: format!("room {} has no live instances", room.room_name),
            });
        } else {
            self.step_player(left.pressed, right.pressed, jump.just_pressed)?;
        }

        if self.pending_room_reset || self.pending_room_transition.is_some() {
            self.apply_pending_room_change()?;
        }

        self.render(host)?;
        Ok(())
    }

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
        self.current_room = Some(self.build_room(room_id)?);
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    fn build_room(&self, room_id: usize) -> Result<RuntimeRoomState, RuntimeCoreError> {
        let room = self
            .room_index
            .get(&room_id)
            .and_then(|index| self.package.rooms.get(*index))
            .ok_or(RuntimeCoreError::RoomMissing(room_id))?;

        let spawn_point = room
            .instances
            .iter()
            .find(|instance| instance.is_checkpoint)
            .or_else(|| room.instances.first())
            .map(|instance| (instance.x, instance.y));

        let mut instances = room
            .instances
            .iter()
            .enumerate()
            .filter_map(|(runtime_id, instance)| {
                let object = self.package.objects.get(instance.object_id as usize)?;
                let (width, height, origin_x, origin_y) = self.sprite_metrics(object);
                Some(RuntimeInstance {
                    runtime_id,
                    instance_id: instance.instance_id,
                    object_id: instance.object_id as usize,
                    object_name: object.name.clone(),
                    x: instance.x,
                    y: instance.y,
                    previous_x: instance.x,
                    previous_y: instance.y,
                    hspeed: 0,
                    vspeed: 0,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    alive: true,
                    solid: instance.is_solid || object.solid,
                    hazard: instance.is_hazard || object.is_hazard.unwrap_or(false),
                    checkpoint: instance.is_checkpoint || object.is_checkpoint.unwrap_or(false),
                    player_candidate: object.is_player,
                })
            })
            .collect::<Vec<_>>();

        let has_player = instances
            .iter()
            .any(|instance| instance.player_candidate && instance.alive && is_preferred_player_name(&instance.object_name));
        if !has_player {
            let preferred_player = self
                .package
                .objects
                .iter()
                .find(|object| object.is_player && is_preferred_player_name(&object.name))
                .or_else(|| self.package.objects.iter().find(|object| object.is_player));

            if let Some(player_object) = preferred_player {
                let (width, height, origin_x, origin_y) = self.sprite_metrics(player_object);
                let (x, y) = spawn_point.unwrap_or((0, 0));
                instances.push(RuntimeInstance {
                    runtime_id: instances.len(),
                    instance_id: -1,
                    object_id: player_object.id,
                    object_name: player_object.name.clone(),
                    x,
                    y,
                    previous_x: x,
                    previous_y: y,
                    hspeed: 0,
                    vspeed: 0,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    alive: true,
                    solid: player_object.solid,
                    hazard: player_object.is_hazard.unwrap_or(false),
                    checkpoint: player_object.is_checkpoint.unwrap_or(false),
                    player_candidate: true,
                });
            }
        }

        Ok(RuntimeRoomState {
            room_id: room.id,
            room_name: room.name.clone(),
            width: room.width,
            height: room.height,
            speed: room.speed,
            playable: room.playable,
            transition_targets: room.transition_targets.clone(),
            spawn_point,
            instances,
        })
    }

    fn sprite_metrics(&self, object: &ObjectDefinition) -> (i32, i32, i32, i32) {
        if object.sprite_index >= 0 {
            if let Some(sprite) = self
                .package
                .resources
                .sprites
                .iter()
                .find(|sprite| sprite.id == object.sprite_index as usize)
            {
                return (
                    sprite.width.max(1) as i32,
                    sprite.height.max(1) as i32,
                    sprite.origin_x,
                    sprite.origin_y,
                );
            }
        }

        (16, 16, 0, 0)
    }

    fn apply_pending_room_change(&mut self) -> Result<(), RuntimeCoreError> {
        if self.pending_room_reset {
            let room_id = self
                .current_room
                .as_ref()
                .map(|room| room.room_id)
                .ok_or(RuntimeCoreError::NoRooms)?;
            self.pending_room_reset = false;
            self.current_room = Some(self.build_room(room_id)?);
            self.reset_player_to_spawn();
            self.status = RuntimeStatus::Ready;
        }

        if let Some(room_id) = self.pending_room_transition.take() {
            self.current_room = Some(self.build_room(room_id)?);
            self.status = RuntimeStatus::Ready;
        }

        Ok(())
    }

    fn reset_player_to_spawn(&mut self) {
        let Some(room) = self.current_room.as_mut() else {
            return;
        };

        let Some((spawn_x, spawn_y)) = room.spawn_point else {
            return;
        };

        if let Some(player) = room
            .instances
            .iter_mut()
            .find(|instance| is_player_instance(instance))
        {
            player.x = spawn_x;
            player.y = spawn_y;
            player.previous_x = spawn_x;
            player.previous_y = spawn_y;
            player.hspeed = 0;
            player.vspeed = 0;
        }
    }

    fn step_player(
        &mut self,
        left_pressed: bool,
        right_pressed: bool,
        jump_just_pressed: bool,
    ) -> Result<(), RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            return Err(RuntimeCoreError::NoRooms);
        };

        let player_index = room.instances.iter().position(is_player_instance);
        let Some(player_index) = player_index else {
            return Ok(());
        };

        let room_width = room.width;
        let room_height = room.height;
        let room_name = room.room_name.clone();
        let solids = room
            .instances
            .iter()
            .filter(|instance| instance.alive && instance.solid)
            .cloned()
            .collect::<Vec<_>>();
        let hazards = room
            .instances
            .iter()
            .filter(|instance| instance.alive && instance.hazard)
            .cloned()
            .collect::<Vec<_>>();

        let room = self.current_room.as_mut().ok_or(RuntimeCoreError::NoRooms)?;
        let player = room
            .instances
            .get_mut(player_index)
            .ok_or(RuntimeCoreError::NoRooms)?;

        player.previous_x = player.x;
        player.previous_y = player.y;

        player.hspeed = match (left_pressed, right_pressed) {
            (true, false) => -RUN_SPEED,
            (false, true) => RUN_SPEED,
            _ => 0,
        };

        let standing_on_solid =
            collides_at(player, player.x, player.y + 1, &solids, Some(player.runtime_id));
        if jump_just_pressed && standing_on_solid {
            player.vspeed = -JUMP_SPEED;
        }

        player.vspeed = (player.vspeed + GRAVITY).min(MAX_FALL_SPEED);

        move_instance_axis(player, &solids, Some(player.runtime_id), Axis::Horizontal, player.hspeed);
        move_instance_axis(player, &solids, Some(player.runtime_id), Axis::Vertical, player.vspeed);

        if collides_at(player, player.x, player.y, &hazards, Some(player.runtime_id)) {
            self.push_diagnostic(
                iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                "runtime-player-died",
                format!("player hit a hazard in {}", room_name),
            );
            self.pending_room_reset = true;
        } else if player_out_of_bounds(player, room_width, room_height) && !room.transition_targets.is_empty() {
            self.pending_room_transition = room.transition_targets.first().copied();
        }

        Ok(())
    }

    fn push_diagnostic(
        &mut self,
        level: iwm_runtime_host::RuntimeDiagnosticLevel,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(iwm_runtime_host::RuntimeDiagnostic {
            level,
            code: code.into(),
            message: message.into(),
        });
    }

    fn build_render_frame(&self) -> Result<RuntimeRenderFrame, RuntimeCoreError> {
        let room = self.current_room.as_ref().ok_or(RuntimeCoreError::NoRooms)?;
        let source_room = self
            .room_index
            .get(&room.room_id)
            .and_then(|index| self.package.rooms.get(*index))
            .ok_or(RuntimeCoreError::RoomMissing(room.room_id))?;

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
                .filter(|layer| layer.visible_on_start && !layer.is_foreground && layer.source_bg >= 0)
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

        commands.extend(source_room.tiles.iter().filter(|tile| tile.source_bg >= 0).map(|tile| {
            RuntimeDrawCommand::DrawTile {
                background_id: tile.source_bg as usize,
                x: tile.x,
                y: tile.y,
                tile_x: tile.tile_x,
                tile_y: tile.tile_y,
                width: tile.width,
                height: tile.height,
                xscale: tile.xscale,
                yscale: tile.yscale,
            }
        }));

        for instance in &room.instances {
            if let Some(object) = self.package.objects.get(instance.object_id) {
                if object.visible && object.sprite_index >= 0 {
                    let sprite = self
                        .package
                        .resources
                        .sprites
                        .iter()
                        .find(|sprite| sprite.id == object.sprite_index as usize);

                    commands.push(RuntimeDrawCommand::DrawSprite {
                        sprite_id: object.sprite_index as usize,
                        frame_index: 0,
                        x: instance.x,
                        y: instance.y,
                        origin_x: sprite.map(|sprite| sprite.origin_x).unwrap_or(0),
                        origin_y: sprite.map(|sprite| sprite.origin_y).unwrap_or(0),
                        xscale: 1.0,
                        yscale: 1.0,
                        angle_degrees: 0.0,
                    });
                    continue;
                }
            }

            commands.push(RuntimeDrawCommand::FillRect {
                x: instance.x - 4,
                y: instance.y - 4,
                width: 8,
                height: 8,
                colour: Rgba8 {
                    r: 96,
                    g: 112,
                    b: 138,
                    a: 255,
                },
            });
        }

        commands.extend(
            source_room
                .backgrounds
                .iter()
                .filter(|layer| layer.visible_on_start && layer.is_foreground && layer.source_bg >= 0)
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

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

fn is_preferred_player_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "player" | "player2" | "playerface" | "obj_player" | "obj_player2" | "obj_playerface"
    )
}

fn is_player_instance(instance: &RuntimeInstance) -> bool {
    instance.player_candidate && instance.alive && is_preferred_player_name(&instance.object_name)
}

fn bounds_at(instance: &RuntimeInstance, x: i32, y: i32) -> (i32, i32, i32, i32) {
    let left = x - instance.origin_x;
    let top = y - instance.origin_y;
    let right = left + instance.width.max(1);
    let bottom = top + instance.height.max(1);
    (left, top, right, bottom)
}

fn collides_at(
    instance: &RuntimeInstance,
    x: i32,
    y: i32,
    others: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
) -> bool {
    let (left, top, right, bottom) = bounds_at(instance, x, y);

    others.iter().any(|other| {
        if !other.alive || ignore_runtime_id == Some(other.runtime_id) {
            return false;
        }

        let (other_left, other_top, other_right, other_bottom) = bounds_at(other, other.x, other.y);
        left < other_right && right > other_left && top < other_bottom && bottom > other_top
    })
}

fn move_instance_axis(
    instance: &mut RuntimeInstance,
    solids: &[RuntimeInstance],
    ignore_runtime_id: Option<usize>,
    axis: Axis,
    delta: i32,
) {
    let step = delta.signum();
    let mut remaining = delta.abs();

    while remaining > 0 {
        let next_x = match axis {
            Axis::Horizontal => instance.x + step,
            Axis::Vertical => instance.x,
        };
        let next_y = match axis {
            Axis::Horizontal => instance.y,
            Axis::Vertical => instance.y + step,
        };

        if collides_at(instance, next_x, next_y, solids, ignore_runtime_id) {
            match axis {
                Axis::Horizontal => instance.hspeed = 0,
                Axis::Vertical => instance.vspeed = 0,
            }
            break;
        }

        instance.x = next_x;
        instance.y = next_y;
        remaining -= 1;
    }
}

fn player_out_of_bounds(instance: &RuntimeInstance, room_width: u32, room_height: u32) -> bool {
    let (left, top, right, bottom) = bounds_at(instance, instance.x, instance.y);
    right < 0 || bottom < 0 || left > room_width as i32 || top > room_height as i32
}


#[cfg(test)]
mod tests {
    use super::*;
    use iwm_runtime_model::{
        AnalysisReport, BackgroundResource, CompatibilityLevel, LogicBlock, LogicOp,
        ObjectEventEntry, ResourceIndex, RoomBackgroundLayer, RoomInstancePlacement,
        RoomTilePlacement, RoomView, RuntimeManifest, ScriptIrFile, SoundResource, SpriteResource,
    };
    use iwm_runtime_host::{ButtonState, HeadlessHost, RuntimeDiagnosticLevel};

    fn sample_package() -> RuntimePackage {
        RuntimePackage {
            manifest: RuntimeManifest {
                format_version: 1,
                package_kind: "runtime-v1".into(),
                source_name: "sample.exe".into(),
                source_hash: "abc123".into(),
                engine_family: "gm8".into(),
                compatibility: CompatibilityLevel::Partial,
                default_room_id: Some(7),
                room_count: 2,
                object_count: 4,
                script_block_count: 1,
                sprite_count: 1,
                background_count: 1,
                sound_count: 0,
                resource_index_path: "resources/index.json".into(),
                warnings: vec![],
            },
            rooms: vec![
                RoomDefinition {
                    id: 7,
                    name: "room7".into(),
                    width: 320,
                    height: 240,
                    speed: 60,
                    persistent: false,
                    backgrounds: vec![RoomBackgroundLayer {
                        visible_on_start: true,
                        is_foreground: false,
                        source_bg: 0,
                        xoffset: 0,
                        yoffset: 0,
                        tile_horz: false,
                        tile_vert: false,
                        hspeed: 0,
                        vspeed: 0,
                        stretch: false,
                    }],
                    views_enabled: false,
                    views: vec![RoomView {
                        visible: true,
                        source_x: 0,
                        source_y: 0,
                        source_w: 320,
                        source_h: 240,
                        port_x: 0,
                        port_y: 0,
                        port_w: 320,
                        port_h: 240,
                        target: -1,
                    }],
                    tiles: vec![RoomTilePlacement {
                        tile_id: 21,
                        source_bg: 0,
                        x: 64,
                        y: 80,
                        tile_x: 0,
                        tile_y: 0,
                        width: 32,
                        height: 32,
                        depth: 100,
                        xscale: 1.0,
                        yscale: 1.0,
                        blend: 0x00ff_ffff,
                    }],
                    instances: vec![
                        RoomInstancePlacement {
                            instance_id: 11,
                            object_id: 0,
                            x: 12,
                            y: 24,
                            xscale: 1.0,
                            yscale: 1.0,
                            angle: 0.0,
                            blend: 0x00ff_ffff,
                            creation_block_id: None,
                            is_solid: false,
                            is_hazard: false,
                            is_checkpoint: false,
                        },
                        RoomInstancePlacement {
                            instance_id: 12,
                            object_id: 1,
                            x: 48,
                            y: 64,
                            xscale: 1.0,
                            yscale: 1.0,
                            angle: 0.0,
                            blend: 0x00ff_ffff,
                            creation_block_id: None,
                            is_solid: false,
                            is_hazard: false,
                            is_checkpoint: false,
                        },
                        RoomInstancePlacement {
                            instance_id: 13,
                            object_id: 2,
                            x: 12,
                            y: 40,
                            xscale: 1.0,
                            yscale: 1.0,
                            angle: 0.0,
                            blend: 0x00ff_ffff,
                            creation_block_id: None,
                            is_solid: true,
                            is_hazard: false,
                            is_checkpoint: false,
                        },
                        RoomInstancePlacement {
                            instance_id: 14,
                            object_id: 3,
                            x: 12,
                            y: 24,
                            xscale: 1.0,
                            yscale: 1.0,
                            angle: 0.0,
                            blend: 0x00ff_ffff,
                            creation_block_id: None,
                            is_solid: false,
                            is_hazard: false,
                            is_checkpoint: true,
                        },
                    ],
                    creation_block_id: None,
                    playable: true,
                    transition_targets: vec![9],
                },
                RoomDefinition {
                    id: 9,
                    name: "room9".into(),
                    width: 160,
                    height: 120,
                    speed: 60,
                    persistent: false,
                    backgrounds: vec![],
                    views_enabled: false,
                    views: vec![],
                    tiles: vec![],
                    instances: vec![],
                    creation_block_id: None,
                    playable: true,
                    transition_targets: vec![],
                },
            ],
            objects: vec![ObjectDefinition {
                id: 0,
                name: "obj_player".into(),
                sprite_index: 0,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: true,
                events: vec![ObjectEventEntry {
                    event_type: 0,
                    sub_event: 0,
                    event_tag: "create".into(),
                    block_id: "object:0:event:0:0".into(),
                    action_count: 0,
                }],
            }, ObjectDefinition {
                id: 1,
                name: "obj_marker".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: false,
                events: vec![],
            }, ObjectDefinition {
                id: 2,
                name: "obj_block".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: false,
                solid: true,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(true),
                is_player: false,
                events: vec![],
            }, ObjectDefinition {
                id: 3,
                name: "obj_checkpoint".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: false,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(true),
                is_player: false,
                events: vec![],
            }],
            scripts: ScriptIrFile {
                format: "iwm-script-ir-v1".into(),
                blocks: vec![LogicBlock {
                    id: "object:0:event:0:0".into(),
                    name: "object event".into(),
                    kind: "object-event".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![LogicOp::Unsupported {
                        reason: "placeholder".into(),
                    }],
                }],
            },
            resources: ResourceIndex {
                sprites: vec![SpriteResource {
                    id: 0,
                    name: "spr_player".into(),
                    origin_x: 0,
                    origin_y: 0,
                    frame_paths: vec![],
                    width: 16,
                    height: 16,
                }],
                backgrounds: vec![BackgroundResource {
                    id: 0,
                    name: "bg_room".into(),
                    width: 320,
                    height: 240,
                    image_path: "resources/backgrounds/0.png".into(),
                }],
                sounds: vec![SoundResource {
                    id: 0,
                    name: "snd_beep".into(),
                    file_path: "resources/audio/0.wav".into(),
                    extension: "wav".into(),
                    preload: false,
                }],
            },
            analysis: AnalysisReport {
                dlls: vec![],
                included_files: vec![],
                warnings: vec![],
                unsupported_features: vec![],
            },
        }
    }

    #[test]
    fn core_loads_default_room_and_instances() {
        let core = RuntimeCore::load(sample_package()).unwrap();

        assert_eq!(core.status(), RuntimeStatus::Ready);
        assert_eq!(core.current_room().map(|room| room.room_id), Some(7));
        assert_eq!(
            core.current_room().map(|room| room.instances.len()),
            Some(4)
        );
        assert!(core.current_room().unwrap().instances[0].player_candidate);
    }

    #[test]
    fn core_ticks_and_submits_a_frame() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.tick(&mut host).unwrap();

        assert_eq!(core.status(), RuntimeStatus::Running);
        assert_eq!(core.tick_count(), 1);
        assert_eq!(host.renderer.submitted_frames.len(), 1);
        assert!(host.renderer.submitted_frames[0]
            .commands
            .iter()
            .any(|command| matches!(command, RuntimeDrawCommand::Present)));
    }

    #[test]
    fn runtime_core_emits_browser_consumable_draw_commands() {
        let package = sample_package();
        let mut core = RuntimeCore::load(package).unwrap();
        let mut host = HeadlessHost::new("runtime-core");

        core.render(&mut host).unwrap();

        let frame = host.renderer.submitted_frames.last().unwrap();
        assert_eq!(frame.room_id, Some(7));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            RuntimeDrawCommand::DrawBackground { background_id: 0, .. }
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            RuntimeDrawCommand::DrawTile {
                background_id: 0,
                width: 32,
                height: 32,
                ..
            }
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            RuntimeDrawCommand::DrawSprite { sprite_id: 0, .. }
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            RuntimeDrawCommand::FillRect { .. }
        )));
    }

    #[test]
    fn core_reports_missing_room() {
        let mut package = sample_package();
        package.manifest.default_room_id = Some(99);

        let error = RuntimeCore::load(package).unwrap_err();
        assert!(matches!(error, RuntimeCoreError::RoomMissing(99)));
    }

    #[test]
    fn core_spawns_a_fallback_player_when_room_has_checkpoint_but_no_player_instance() {
        let mut package = sample_package();
        package.rooms[0].instances.retain(|instance| instance.object_id != 0);
        package.rooms[0].instances[0].is_checkpoint = true;

        let core = RuntimeCore::load(package).unwrap();
        let room = core.current_room().unwrap();

        assert!(room.instances.iter().any(|instance| instance.player_candidate));
        assert!(room
            .instances
            .iter()
            .any(|instance| instance.player_candidate && instance.instance_id == -1));
    }

    #[test]
    fn core_ignores_player_start_markers_when_deciding_whether_a_room_has_a_player() {
        let mut package = sample_package();
        package.rooms[0].instances.retain(|instance| instance.object_id != 0);
        package.rooms[0].instances[0].is_checkpoint = true;
        package.rooms[0].instances.push(RoomInstancePlacement {
            instance_id: 99,
            object_id: 2,
            x: 24,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: true,
        });
        package.objects.push(ObjectDefinition {
            id: 2,
            name: "playerStart".into(),
            sprite_index: -1,
            parent_index: -1,
            depth: -10,
            persistent: false,
            visible: false,
            solid: false,
            mask_index: -1,
            is_hazard: Some(false),
            is_checkpoint: Some(true),
            is_player: true,
            events: vec![],
        });

        let core = RuntimeCore::load(package).unwrap();
        let room = core.current_room().unwrap();

        assert!(room.instances.iter().any(|instance| {
            instance.instance_id == -1 && instance.object_id == 0 && instance.player_candidate
        }));
    }

    #[test]
    fn core_emits_idle_diagnostic_when_no_input_is_active() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.tick(&mut host).unwrap();

        assert!(core
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "runtime-idle"
                && matches!(diagnostic.level, RuntimeDiagnosticLevel::Info)));
    }

    #[test]
    fn core_moves_player_with_left_and_right_input() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        host.input.set_button_state(
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        );
        core.tick(&mut host).unwrap();
        let after_right = core.current_room().unwrap();
        let player = after_right.instances.iter().find(|instance| instance.player_candidate).unwrap();
        let right_x = player.x;
        assert!(right_x > 12);

        host.input.replace_button_states([(
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        )]);
        core.tick(&mut host).unwrap();
        let after_left = core.current_room().unwrap();
        let player = after_left.instances.iter().find(|instance| instance.player_candidate).unwrap();
        assert!(player.x <= right_x);
    }

    #[test]
    fn core_jumps_when_on_spawn_and_jump_is_pressed() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        host.input.set_button_state(
            RuntimeButton::Keyboard(0x20),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        );
        core.tick(&mut host).unwrap();

        let room = core.current_room().unwrap();
        let player = room.instances.iter().find(|instance| instance.player_candidate).unwrap();
        assert!(player.y <= 24);
    }

    #[test]
    fn core_stops_player_when_moving_into_a_solid() {
        let mut package = sample_package();
        package.rooms[0].instances.push(RoomInstancePlacement {
            instance_id: 15,
            object_id: 2,
            x: 28,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
        });

        let mut core = RuntimeCore::load(package).unwrap();
        let mut host = HeadlessHost::new("sandbox");
        host.input.set_button_state(
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        );

        core.tick(&mut host).unwrap();

        let room = core.current_room().unwrap();
        let player = room.instances.iter().find(|instance| instance.player_candidate).unwrap();
        assert_eq!(player.x, 12);
    }

    #[test]
    fn core_resets_player_back_to_spawn() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        host.input.set_button_state(
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        );
        core.tick(&mut host).unwrap();

        host.input.replace_button_states([(
            RuntimeButton::Keyboard(0x52),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        )]);
        core.tick(&mut host).unwrap();

        let room = core.current_room().unwrap();
        let player = room.instances.iter().find(|instance| instance.player_candidate).unwrap();
        assert_eq!((player.x, player.y), (12, 24));
    }

    #[test]
    fn core_transitions_to_target_room_when_requested() {
        let mut package = sample_package();
        package.rooms[0].transition_targets = vec![1];
        let mut core = RuntimeCore::load(package).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.request_room_transition(9);
        core.tick(&mut host).unwrap();
        assert_eq!(core.snapshot().room_id, Some(9));
    }

    #[test]
    fn core_emits_hazard_diagnostic_and_requests_reset() {
        let mut package = sample_package();
        package.rooms[0].instances.push(RoomInstancePlacement {
            instance_id: 13,
            object_id: 1,
            x: 12,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: true,
            is_checkpoint: false,
        });
        let mut core = RuntimeCore::load(package).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.tick(&mut host).unwrap();

        assert!(core
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "runtime-player-died"));
        assert_eq!(core.snapshot().status, RuntimeStatus::Ready);
    }
}
