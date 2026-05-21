use crate::{RuntimeAudioHost, RuntimeHostError, RuntimeSoundMode};

#[derive(Debug, Default)]
pub struct NoopAudioHost {
    pub played: Vec<(i32, RuntimeSoundMode)>,
    pub stopped: Vec<i32>,
}

impl RuntimeAudioHost for NoopAudioHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.played.push((sound_id, mode));
        Ok(())
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.stopped.push(sound_id);
        Ok(())
    }
}
