#[allow(noop_method_call, unused_assignments)]
pub mod base64 {

    use crate::{
        pipeline_module::pipeline::{PipelineDirection, PipelineStep},
        BoxedClone, IOError, Read,
    };
    use base64::{
        alphabet,
        engine::{general_purpose::NO_PAD, GeneralPurpose},
        Engine as _,
    };
    use std::{collections::VecDeque, io::Write};

    pub const B64_ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::BIN_HEX, NO_PAD);
    pub const NEW_LINE: &[u8] = &[b'\n'; 1];
    pub struct Base64 {
        forward_buffer: Vec<u8>,
        backward_buffer: Vec<u8>,
        work_mode: PipelineDirection,
        pipeline_direction: PipelineDirection,
    }

    impl PipelineStep for Base64 {
        fn len(&mut self) -> std::io::Result<usize> {
            // Ok(self.forward_buffer.len())
            match self.pipeline_direction {
                PipelineDirection::Forward => Ok(self.forward_buffer.len()),
                PipelineDirection::Backward => Ok(self.backward_buffer.len()),
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            self.pipeline_direction = direction;
        }

        fn start(&mut self) {}
    }

    impl BoxedClone for Base64 {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            match self.work_mode {
                PipelineDirection::Forward => Box::new(Base64::new(Some("fw"))),
                PipelineDirection::Backward => Box::new(Base64::new(Some("bw"))),
            }
        }
    }

    impl Read for Base64 {
        fn read(&mut self) -> Result<Vec<u8>, IOError> {
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    let result = self.forward_buffer.clone();
                    self.forward_buffer.clear();
                    Ok(result)
                }
                PipelineDirection::Backward => {
                    let result = self.backward_buffer.clone();
                    self.backward_buffer.clear();
                    Ok(result)
                }
            }
        }
    }

    impl Write for Base64 {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut local_buf = buf.clone();
            if buf.ends_with(NEW_LINE) {
                local_buf = &buf[0..buf.len() - 1];
            }
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64_ENGINE.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.forward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let b64 = B64_ENGINE.decode(local_buf).unwrap();
                        self.forward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
                PipelineDirection::Backward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64_ENGINE.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.backward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let s = std::str::from_utf8(local_buf).unwrap();
                        let b64 = B64_ENGINE.decode(s).unwrap();
                        self.backward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Base64 {
        pub fn new(config: Option<&str>) -> Base64 {
            let mut work_mode: PipelineDirection = PipelineDirection::Forward;
            match config {
                Some("fw") => work_mode = PipelineDirection::Forward,
                Some("bw") => work_mode = PipelineDirection::Backward,
                Some(_) | None => {
                    panic!("Base64Encoder : Unknown Work Mode")
                }
            }
            Base64 {
                forward_buffer: vec![0; 0],
                backward_buffer: vec![0; 0],
                work_mode: work_mode,
                pipeline_direction: PipelineDirection::Forward,
            }
        }
    }
}
