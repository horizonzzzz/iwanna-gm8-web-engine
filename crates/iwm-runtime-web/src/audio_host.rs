use std::collections::HashSet;

use iwm_runtime_host::{RuntimeAudioHost, RuntimeHostError, RuntimeSoundMode};

#[derive(Debug, Default)]
pub struct WebAudioHost {
    events: Vec<String>,
    playing: HashSet<i32>,
}

impl WebAudioHost {
    pub fn events(&self) -> &[String] {
        &self.events
    }
}

impl RuntimeAudioHost for WebAudioHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.events
            .push(format!("play:{sound_id}:{}", sound_mode_label(mode)));
        self.playing.insert(sound_id);
        emit_play_sound(sound_id, mode);
        Ok(())
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.events.push(format!("stop:{sound_id}"));
        self.playing.remove(&sound_id);
        emit_stop_sound(sound_id);
        Ok(())
    }

    fn stop_all_sounds(&mut self) -> Result<(), RuntimeHostError> {
        self.events.push("stop-all".into());
        self.playing.clear();
        emit_stop_all_sounds();
        Ok(())
    }

    fn is_sound_playing(&self, sound_id: i32) -> Result<bool, RuntimeHostError> {
        Ok(query_is_sound_playing(sound_id).unwrap_or_else(|| self.playing.contains(&sound_id)))
    }
}

fn sound_mode_label(mode: RuntimeSoundMode) -> &'static str {
    match mode {
        RuntimeSoundMode::Once => "once",
        RuntimeSoundMode::Loop => "loop",
    }
}

#[cfg(target_arch = "wasm32")]
fn emit_play_sound(sound_id: i32, mode: RuntimeSoundMode) {
    unsafe {
        iwm_host_play_sound(sound_id, sound_mode_code(mode));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn emit_play_sound(_sound_id: i32, _mode: RuntimeSoundMode) {}

#[cfg(target_arch = "wasm32")]
fn emit_stop_sound(sound_id: i32) {
    unsafe {
        iwm_host_stop_sound(sound_id);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn emit_stop_sound(_sound_id: i32) {}

#[cfg(target_arch = "wasm32")]
fn emit_stop_all_sounds() {
    unsafe {
        iwm_host_stop_all_sounds();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn emit_stop_all_sounds() {}

#[cfg(target_arch = "wasm32")]
fn query_is_sound_playing(sound_id: i32) -> Option<bool> {
    Some(unsafe { iwm_host_is_sound_playing(sound_id) != 0 })
}

#[cfg(not(target_arch = "wasm32"))]
fn query_is_sound_playing(_sound_id: i32) -> Option<bool> {
    None
}

#[cfg(target_arch = "wasm32")]
fn sound_mode_code(mode: RuntimeSoundMode) -> i32 {
    match mode {
        RuntimeSoundMode::Once => 0,
        RuntimeSoundMode::Loop => 1,
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn iwm_host_play_sound(sound_id: i32, mode: i32);
    fn iwm_host_stop_sound(sound_id: i32);
    fn iwm_host_stop_all_sounds();
    fn iwm_host_is_sound_playing(sound_id: i32) -> i32;
}
