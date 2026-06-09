use std::collections::HashSet;

use crate::{RuntimeAudioHost, RuntimeHostError, RuntimeSoundMode};

#[derive(Debug, Default)]
pub struct NoopAudioHost {
    pub played: Vec<(i32, RuntimeSoundMode)>,
    pub stopped: Vec<i32>,
    pub stopped_all_count: usize,
    pub playing: HashSet<i32>,
}

impl RuntimeAudioHost for NoopAudioHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.played.push((sound_id, mode));
        self.playing.insert(sound_id);
        Ok(())
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.stopped.push(sound_id);
        self.playing.remove(&sound_id);
        Ok(())
    }

    fn stop_all_sounds(&mut self) -> Result<(), RuntimeHostError> {
        self.stopped_all_count += 1;
        self.playing.clear();
        Ok(())
    }

    fn is_sound_playing(&self, sound_id: i32) -> Result<bool, RuntimeHostError> {
        Ok(self.playing.contains(&sound_id))
    }
}
