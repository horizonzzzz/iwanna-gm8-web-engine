use crate::{RuntimeHostError, RuntimeRenderFrame, RuntimeRenderHost};

#[derive(Debug, Default)]
pub struct NullRenderHost {
    pub submitted_frames: Vec<RuntimeRenderFrame>,
}

impl RuntimeRenderHost for NullRenderHost {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError> {
        self.submitted_frames.push(frame);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rgba8, RuntimeDrawCommand};

    #[test]
    fn null_render_host_collects_frames() {
        let mut renderer = NullRenderHost::default();
        renderer
            .submit_frame(RuntimeRenderFrame {
                tick: 0,
                room_id: None,
                width: 320,
                height: 240,
                commands: vec![
                    RuntimeDrawCommand::Clear {
                        colour: Rgba8 {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        },
                    },
                    RuntimeDrawCommand::Present,
                ],
            })
            .unwrap();

        assert_eq!(renderer.submitted_frames.len(), 1);
        assert_eq!(renderer.submitted_frames[0].commands.len(), 2);
    }
}
