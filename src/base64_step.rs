pub mod base64 {

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};
    use base64::{
        alphabet,
        engine::{
            general_purpose::{self, NO_PAD},
            GeneralPurpose,
        },
        Engine as _,
    };
    use std::io::{Read, Write};

    pub const B64Engine: GeneralPurpose = GeneralPurpose::new(&alphabet::BIN_HEX, NO_PAD);
    pub const NewLine: &[u8] = &[b'\n'; 1];
    pub struct Base64 {
        forward_buffer: Vec<u8>,
        backward_buffer: Vec<u8>,
        work_mode: PipelineDirection,
        pipeline_direction: PipelineDirection,
    }

    impl PipelineStep for Base64 {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Middle
        }

        fn len(&self) -> std::io::Result<usize> {
            // Ok(self.forward_buffer.len())
            match self.pipeline_direction {
                PipelineDirection::Forward => Ok(self.forward_buffer.len()),
                PipelineDirection::Backward => Ok(self.backward_buffer.len()),
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            // println!("{}", direction);
            self.pipeline_direction = direction;
        }
    }

    impl Read for Base64 {
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

            // let length = std::cmp::min(self.forward_buffer.len(), buf.len());
            // let size = buf.write(&self.forward_buffer[0..length]).unwrap();
            // self.forward_buffer.drain(0..size);
            // Ok(size)
        }
    }

    impl Write for Base64 {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut local_buf = buf.clone();
            if buf.ends_with(NewLine) {
                local_buf = &buf[0..buf.len() - 1];
            }
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64Engine.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.forward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let b64 = B64Engine.decode(local_buf).unwrap();
                        self.forward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
                PipelineDirection::Backward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64Engine.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.backward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let len = buf.len();
                        let s = std::str::from_utf8(local_buf).unwrap();
                        let b64 = B64Engine.decode(s).unwrap();
                        self.backward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
            }
            // if self.work_mode == self.pipeline_direction {
            //     let b64 = general_purpose::STANDARD.encode(buf);
            //     let data: &[u8] = b64.as_bytes();
            //     self.forward_buffer.extend(data);
            //     Ok(data.len())
            // } else {
            //     self.forward_buffer.extend(buf);
            //     Ok(buf.len())
            // }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Base64 {
        pub fn new(config: Option<&str>, mut buffer_size: Option<usize>) -> Base64 {
            if (buffer_size == None) {
                buffer_size = Some(1024);
            }

            let mut work_mode = PipelineDirection::Forward;
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

    //     use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};
    //     use base64::{engine::general_purpose, Engine as _};
    //     use std::io::{Read, Write};

    //     pub struct Base64Encoder {
    //         buffer: Vec<u8>,
    //         work_mode: PipelineDirection,
    //         pipeline_direction: PipelineDirection,
    //     }

    //     impl PipelineStep for Base64Encoder {
    //         fn get_step_type(&self) -> PipelineStepType {
    //             PipelineStepType::Middle
    //         }

    //         fn len(&self) -> std::io::Result<usize> {
    //             Ok(self.buffer.len())
    //         }

    //         fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
    //             // println!("{}", direction);
    //             self.pipeline_direction = direction;
    //         }
    //     }

    //     impl Read for Base64Encoder {
    //         fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
    //             let length = std::cmp::min(self.buffer.len(), buf.len());
    //             let size = buf.write(&self.buffer[0..length]).unwrap();
    //             self.buffer.drain(0..size);
    //             Ok(size)
    //         }
    //     }

    //     impl Write for Base64Encoder {
    //         fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    //             if self.work_mode == self.pipeline_direction {
    //                 let b64 = general_purpose::STANDARD.encode(buf);
    //                 let data: &[u8] = b64.as_bytes();
    //                 self.buffer.extend(data);
    //                 Ok(data.len())
    //             } else {
    //                 self.buffer.extend(buf);
    //                 Ok(buf.len())
    //             }
    //         }

    //         fn flush(&mut self) -> std::io::Result<()> {
    //             Ok(())
    //         }
    //     }

    //     impl Base64Encoder {
    //         pub fn new(config: Option<&str>, mut buffer_size: Option<usize>) -> Base64Encoder {
    //             if (buffer_size == None) {
    //                 buffer_size = Some(1024);
    //             }

    //             let mut work_mode = PipelineDirection::Forward;
    //             match config {
    //                 Some("fw") => work_mode = PipelineDirection::Forward,
    //                 Some("bw") => work_mode = PipelineDirection::Backward,
    //                 Some(_) | None => {
    //                     panic!("Base64Encoder : Unknown Work Mode")
    //                 }
    //             }
    //             Base64Encoder {
    //                 buffer: vec![0; 0],
    //                 work_mode: work_mode,
    //                 pipeline_direction: PipelineDirection::Forward,
    //             }
    //         }
    //     }

    //     pub struct Base64Decoder {
    //         buffer: Vec<u8>,
    //         base64_buffer: Vec<u8>,
    //         work_mode: PipelineDirection,
    //         pipeline_direction: PipelineDirection,
    //     }

    //     impl PipelineStep for Base64Decoder {
    //         fn get_step_type(&self) -> PipelineStepType {
    //             PipelineStepType::Middle
    //         }

    //         fn len(&self) -> std::io::Result<usize> {
    //             // if self.base64_buffer.len() > 0 {
    //             //     Ok(self.base64_buffer.len())
    //             // } else {
    //             //     Ok(self.buffer.len())
    //             // }
    //             Ok(self.buffer.len())
    //         }

    //         fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
    //             // println!("{}", direction);
    //             self.pipeline_direction = direction;
    //         }
    //     }

    //     impl Read for Base64Decoder {
    //         fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
    //             if (self.base64_buffer.len() == 0) {
    //                 let data_str = String::from_utf8(self.buffer.clone()).unwrap();
    //                 self.base64_buffer = general_purpose::STANDARD.decode(data_str).unwrap();
    //                 self.buffer.clear();
    //             }
    //             let length = std::cmp::min(self.base64_buffer.len(), buf.len());
    //             let size = buf.write(&self.base64_buffer[0..length]).unwrap();
    //             self.base64_buffer.drain(0..size);
    //             Ok(size)

    //             // let length = std::cmp::min(self.buffer.len(), buf.len());
    //             // let size = buf.write(&self.buffer[0..length]).unwrap();
    //             // self.buffer.drain(0..size);
    //             // Ok(size)
    //         }
    //     }

    //     impl Write for Base64Decoder {
    //         fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    //             let b64 = general_purpose::STANDARD.encode(buf);
    //             self.buffer.extend(b64.as_bytes());
    //             Ok(buf.len())

    //             // if self.work_mode == self.pipeline_direction {
    //             //     let b64 = general_purpose::STANDARD.decode(buf).unwrap();
    //             //     self.buffer.extend(b64.as_slice());
    //             //     Ok(b64.len())
    //             // } else {
    //             //     self.buffer.extend(buf);
    //             //     Ok(buf.len())
    //             // }
    //         }

    //         fn flush(&mut self) -> std::io::Result<()> {
    //             Ok(())
    //         }
    //     }

    //     impl Base64Decoder {
    //         pub fn new(config: Option<&str>, mut buffer_size: Option<usize>) -> Base64Decoder {
    //             if buffer_size == None {
    //                 buffer_size = Some(1024);
    //             }

    //             let mut work_mode = PipelineDirection::Forward;
    //             match config {
    //                 Some("fw") => work_mode = PipelineDirection::Forward,
    //                 Some("bw") => work_mode = PipelineDirection::Backward,
    //                 Some(_) | None => {
    //                     panic!("Base64Encoder : Unknown Work Mode")
    //                 }
    //             }

    //             Base64Decoder {
    //                 buffer: vec![0; 0],
    //                 base64_buffer: vec![0; 0],
    //                 work_mode: work_mode,
    //                 pipeline_direction: PipelineDirection::Forward,
    //             }
    //         }
    //     }
}
