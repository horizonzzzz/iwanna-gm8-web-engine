use std::convert::TryInto;

use crate::{
    BridgeDrawCommand, BridgeFrameSnapshot, BridgePlayerSnapshot, BridgeRgba8, BridgeSnapshot,
    BridgeStepResult, WebInputState,
};

const MAGIC: u32 = 0x424d5749;
const VERSION: u16 = 1;
const INPUT_KIND: u16 = 1;
const STEP_RESULT_KIND: u16 = 2;

pub fn decode_web_input_state_from_buffer(bytes: &[u8]) -> Result<WebInputState, String> {
    let mut reader = BinaryReader::new(bytes);
    reader.expect_header(INPUT_KIND)?;
    let flags = reader.read_u16()?;
    let _reserved = reader.read_u16()?;
    Ok(WebInputState {
        left: flags & 0b0000_0001 != 0,
        right: flags & 0b0000_0010 != 0,
        jump: flags & 0b0000_0100 != 0,
        jump_pressed: flags & 0b0000_1000 != 0,
        jump_released: flags & 0b0001_0000 != 0,
        restart: flags & 0b0010_0000 != 0,
        keys_held: reader.read_u16_vec()?,
        keys_pressed: reader.read_u16_vec()?,
        keys_released: reader.read_u16_vec()?,
    })
}

pub fn encode_bridge_step_result_to_buffer(step: &BridgeStepResult) -> Result<Vec<u8>, String> {
    let mut writer = BinaryWriter::new();
    writer.write_header(STEP_RESULT_KIND);
    writer.write_snapshot(&step.snapshot)?;
    writer.write_frame(&step.frame)?;
    Ok(writer.into_bytes())
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn expect_header(&mut self, expected_kind: u16) -> Result<(), String> {
        let magic = self.read_u32()?;
        if magic != MAGIC {
            return Err("invalid bridge buffer magic".into());
        }
        let version = self.read_u16()?;
        if version != VERSION {
            return Err(format!("unsupported bridge buffer version: {version}"));
        }
        let kind = self.read_u16()?;
        if kind != expected_kind {
            return Err(format!("unexpected bridge buffer kind: {kind}"));
        }
        Ok(())
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| "bridge buffer offset overflow".to_string())?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "bridge buffer ended unexpectedly".to_string())?;
        self.offset = end;
        slice
            .try_into()
            .map_err(|_| "bridge buffer read length mismatch".to_string())
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.read_exact()?))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.read_exact()?))
    }

    fn read_u16_vec(&mut self) -> Result<Vec<u16>, String> {
        let count = self.read_u32()? as usize;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_u16()?);
        }
        Ok(values)
    }
}

struct BinaryWriter {
    bytes: Vec<u8>,
}

impl BinaryWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn write_header(&mut self, kind: u16) {
        self.write_u32(MAGIC);
        self.write_u16(VERSION);
        self.write_u16(kind);
    }

    fn write_snapshot(&mut self, snapshot: &BridgeSnapshot) -> Result<(), String> {
        self.write_string(&snapshot.status)?;
        self.write_u64(snapshot.tick);
        self.write_option_u32(snapshot.room_id)?;
        self.write_option_string(snapshot.room_name.as_deref())?;
        self.write_option_u32(snapshot.room_speed.map(|value| value as usize))?;
        self.write_u32(usize_to_u32(snapshot.instance_count, "instance_count")?);
        self.write_player(snapshot.player.as_ref())?;
        self.write_u16(snapshot.input_trace.jump_button_key);
        self.write_bool(snapshot.input_trace.jump_pressed);
        self.write_bool(snapshot.input_trace.jump_just_pressed);
        self.write_bool(snapshot.input_trace.jump_just_released);
        self.write_string_array(&snapshot.input_trace.active_keys)?;
        self.write_u64(snapshot.tick_phases.input_diag_nanos);
        self.write_u64(snapshot.tick_phases.step_events_nanos);
        self.write_u64(snapshot.tick_phases.view_sync_nanos);
        self.write_u64(snapshot.tick_phases.player_movement_nanos);
        self.write_u64(snapshot.tick_phases.collision_events_nanos);
        self.write_u64(snapshot.tick_phases.alarms_nanos);
        self.write_u64(snapshot.tick_phases.keyboard_events_nanos);
        self.write_u64(snapshot.tick_phases.render_submit_nanos);
        self.write_u64(snapshot.tick_phases.total_nanos);
        self.write_string_array(&snapshot.diagnostics)
    }

    fn write_player(&mut self, player: Option<&BridgePlayerSnapshot>) -> Result<(), String> {
        let Some(player) = player else {
            self.write_bool(false);
            return Ok(());
        };
        self.write_bool(true);
        self.write_u32(usize_to_u32(player.runtime_id, "player.runtime_id")?);
        self.write_i32(player.instance_id);
        self.write_u32(usize_to_u32(player.object_id, "player.object_id")?);
        self.write_string(&player.object_name)?;
        self.write_f64(player.x);
        self.write_f64(player.y);
        self.write_f64(player.hspeed);
        self.write_f64(player.vspeed);
        self.write_bool(player.facing_left);
        self.write_bool(player.alive);
        self.write_bool(player.jump.grounded);
        self.write_bool(player.jump.active);
        self.write_u32(player.jump.hold_frames);
        self.write_bool(player.jump.cut_applied);
        Ok(())
    }

    fn write_frame(&mut self, frame: &BridgeFrameSnapshot) -> Result<(), String> {
        self.write_u64(frame.tick);
        self.write_option_u32(frame.room_id)?;
        self.write_u32(frame.width);
        self.write_u32(frame.height);
        self.write_u32(usize_to_u32(frame.commands.len(), "frame.commands")?);
        for command in &frame.commands {
            self.write_command(command)?;
        }
        Ok(())
    }

    fn write_command(&mut self, command: &BridgeDrawCommand) -> Result<(), String> {
        match command {
            BridgeDrawCommand::Clear { colour } => {
                self.write_u8(0);
                self.write_rgba(*colour);
            }
            BridgeDrawCommand::DrawBackground {
                background_id,
                x,
                y,
                stretch,
                tile_horz,
                tile_vert,
                is_foreground,
            } => {
                self.write_u8(1);
                self.write_u32(usize_to_u32(*background_id, "background_id")?);
                self.write_i32(*x);
                self.write_i32(*y);
                self.write_bool(*stretch);
                self.write_bool(*tile_horz);
                self.write_bool(*tile_vert);
                self.write_bool(*is_foreground);
            }
            BridgeDrawCommand::DrawTile {
                background_id,
                x,
                y,
                tile_x,
                tile_y,
                width,
                height,
                xscale,
                yscale,
            } => {
                self.write_u8(2);
                self.write_u32(usize_to_u32(*background_id, "background_id")?);
                self.write_i32(*x);
                self.write_i32(*y);
                self.write_u32(*tile_x);
                self.write_u32(*tile_y);
                self.write_u32(*width);
                self.write_u32(*height);
                self.write_f64(*xscale);
                self.write_f64(*yscale);
            }
            BridgeDrawCommand::DrawSprite {
                sprite_id,
                frame_index,
                x,
                y,
                origin_x,
                origin_y,
                xscale,
                yscale,
                alpha,
                angle_degrees,
            } => {
                self.write_u8(3);
                self.write_u32(usize_to_u32(*sprite_id, "sprite_id")?);
                self.write_u32(usize_to_u32(*frame_index, "frame_index")?);
                self.write_i32(*x);
                self.write_i32(*y);
                self.write_i32(*origin_x);
                self.write_i32(*origin_y);
                self.write_f64(*xscale);
                self.write_f64(*yscale);
                self.write_f64(*alpha);
                self.write_f64(*angle_degrees);
            }
            BridgeDrawCommand::FillRect {
                x,
                y,
                width,
                height,
                colour,
            } => {
                self.write_u8(4);
                self.write_i32(*x);
                self.write_i32(*y);
                self.write_u32(*width);
                self.write_u32(*height);
                self.write_rgba(*colour);
            }
            BridgeDrawCommand::DrawText {
                text,
                x,
                y,
                size,
                font_name,
                font_bold,
                font_italic,
                colour,
                align,
            } => {
                self.write_u8(5);
                self.write_string(text)?;
                self.write_i32(*x);
                self.write_i32(*y);
                self.write_u32(*size);
                self.write_option_string(font_name.as_deref())?;
                self.write_bool(*font_bold);
                self.write_bool(*font_italic);
                self.write_rgba(*colour);
                self.write_string(align)?;
            }
            BridgeDrawCommand::Present => {
                self.write_u8(6);
            }
        }
        Ok(())
    }

    fn write_string_array(&mut self, values: &[String]) -> Result<(), String> {
        self.write_u32(usize_to_u32(values.len(), "string array")?);
        for value in values {
            self.write_string(value)?;
        }
        Ok(())
    }

    fn write_option_u32(&mut self, value: Option<usize>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_bool(true);
                self.write_u32(usize_to_u32(value, "optional u32")?);
            }
            None => self.write_bool(false),
        }
        Ok(())
    }

    fn write_option_string(&mut self, value: Option<&str>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_bool(true);
                self.write_string(value)?;
            }
            None => self.write_bool(false),
        }
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), String> {
        self.write_u32(usize_to_u32(value.len(), "string byte length")?);
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn write_rgba(&mut self, colour: BridgeRgba8) {
        self.write_u8(colour.r);
        self.write_u8(colour.g);
        self.write_u8(colour.b);
        self.write_u8(colour.a);
    }

    fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn usize_to_u32(value: usize, field: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{field} does not fit in bridge buffer u32"))
}
