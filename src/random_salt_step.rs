#[allow(noop_method_call, unused_assignments)]
pub mod random_salt_step {
    use crate::{
        pipeline_module::pipeline::{PipelineDirection, PipelineStep},
        BoxedClone,
    };
    use openssl::string;
    use rand::Rng;
    use std::{
        collections::VecDeque,
        io::{Read, Write},
    };

    pub struct RSult {
        salt_lengh: usize,
        forward_buffer: Vec<u8>,
        backward_buffer: Vec<u8>,
        work_mode: PipelineDirection,
        pipeline_direction: PipelineDirection,
    }

    impl PipelineStep for RSult {
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

    impl BoxedClone for RSult {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            let mut config : String = String::new();
            match self.work_mode {
                PipelineDirection::Forward => config.push_str("fw-"),
                PipelineDirection::Backward => config.push_str("bw-"),
            }
            config.push_str(self.salt_lengh.to_string().as_str());
            Box::new(RSult::new(Some(config.as_str())))
        }
    }

    impl Read for RSult {
        fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    let length = std::cmp::min(self.forward_buffer.len(), buf.len());
                    let size = buf.write(&self.forward_buffer[0..length]).unwrap();
                    self.forward_buffer.drain(0..size);
                    Ok(size)
                }
                PipelineDirection::Backward => {
                    let length = std::cmp::min(self.backward_buffer.len(), buf.len());
                    let size = buf.write(&self.backward_buffer[0..length]).unwrap();
                    self.backward_buffer.drain(0..size);
                    Ok(size)
                }
            }
        }
    }

    impl Write for RSult {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    if self.work_mode == self.pipeline_direction {
                        let mut rng = rand::thread_rng();
                        let mut rbytes = vec![0u8; self.salt_lengh as usize];
                        rng.fill(rbytes.as_mut_slice());
                        self.forward_buffer.extend(rbytes);
                        self.forward_buffer.extend(buf);
                        Ok(self.forward_buffer.len())
                    } else {
                        self.forward_buffer.extend(buf[self.salt_lengh..].to_vec());
                        Ok(buf.len() - self.salt_lengh)
                        // self.forward_buffer.extend(buf);
                        // Ok(buf.len())
                    }
                }
                PipelineDirection::Backward => {
                    if self.work_mode == self.pipeline_direction {
                        let mut rng = rand::thread_rng();
                        let mut rbytes = vec![0u8; self.salt_lengh as usize];
                        rng.fill(rbytes.as_mut_slice());
                        self.backward_buffer.extend(rbytes);
                        self.backward_buffer.extend(buf);
                        Ok(self.backward_buffer.len())
                    } else {
                        self.backward_buffer.extend(buf[self.salt_lengh..].to_vec());
                        Ok(buf.len() - self.salt_lengh)
                        // self.backward_buffer.extend(buf);
                        // Ok(buf.len())
                    }
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl RSult {
        pub fn new(config: Option<&str>) -> RSult {
            let mut work_mode: PipelineDirection = PipelineDirection::Forward;
            let mut salt_length: usize = 0;

            match config {
                Some(value) => {
                    let config: Vec<String> = value.split("-").map(|s| s.to_string()).collect();
                    if config.len() != 2 {
                        panic!("random_salt_step: invalid config ");
                    } else {
                        match config[0].as_str() {
                            "fw" => work_mode = PipelineDirection::Forward,
                            "bw" => work_mode = PipelineDirection::Backward,
                            _ => {
                                panic!("random_salt_step : Unknown Work Mode")
                            }
                        }

                        match config[1].parse::<usize>() {
                            Ok(value) => {
                                salt_length = value;
                            }
                            Err(e) => {
                                panic!("random_salt_step: salt length error : {}", e);
                            }
                        }
                    }
                }
                None => {
                    panic!("random_salt_step : Empty Config")
                }
            }

            RSult {
                salt_lengh: salt_length,
                forward_buffer: vec![0; 0],
                backward_buffer: vec![0; 0],
                work_mode: work_mode,
                pipeline_direction: PipelineDirection::Forward,
            }
        }
    }
}
