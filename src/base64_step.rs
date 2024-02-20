pub mod base64_encode {
    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType};
    use base64::{engine::general_purpose, Engine as _};
    use std::io::{Read, Write};

    pub struct Base64Encoder {
        buffer: Vec<u8>,
    }

    impl PipelineStep for Base64Encoder {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Middle
        }
        
        fn len(&self) -> std::io::Result<usize> {
            Ok(self.buffer.len())
        }
    }

    impl Read for Base64Encoder {
        fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
            let length = std::cmp::min(self.buffer.len(), buf.len());
            let size = buf.write(&self.buffer[0..length]).unwrap();
            self.buffer.drain(0..size);
            Ok(size)
        }
    }

    impl Write for Base64Encoder {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let b64 = general_purpose::STANDARD.encode(buf);
            let data: &[u8] = b64.as_bytes();
            self.buffer.extend(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Base64Encoder {
        pub fn new(mut buffer_size: Option<usize>) -> Base64Encoder {
            if(buffer_size == None){
                buffer_size = Some(1024);
            }
            Base64Encoder {
                buffer: vec![0; 0],
            }
        }
    }
}

pub mod base64_decode {
    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType};
    use base64::{engine::general_purpose, Engine as _};
    use std::io::{Read, Write};

    pub struct Base64Decoder {
        buffer: Vec<u8>,
        base64_buffer: Vec<u8>,
    }

    impl PipelineStep for Base64Decoder {
        fn get_step_type(&self) -> PipelineStepType {
            todo!()
        }

        fn len(&self) -> std::io::Result<usize> {
            if self.base64_buffer.len() > 0 {
                Ok(self.base64_buffer.len())
            }else {
                Ok(self.buffer.len())
            }
        }
    }

    impl Read for Base64Decoder {
        fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
            if (self.base64_buffer.len() == 0) {
                let data_str = String::from_utf8(self.buffer.clone()).unwrap();
                self.base64_buffer = general_purpose::STANDARD.decode(data_str).unwrap();
                self.buffer.clear();
            }
            let length = std::cmp::min(self.base64_buffer.len(), buf.len());
            let size = buf.write(&self.base64_buffer[0..length]).unwrap();
            self.base64_buffer.drain(0..size);
            Ok(size)
        }
    }

    impl Write for Base64Decoder {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let b64 = general_purpose::STANDARD.encode(buf);
            self.buffer.extend(b64.as_bytes());
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            todo!()
        }
    }

    impl Base64Decoder {
        pub fn new(mut buffer_size: Option<usize>) -> Base64Decoder {
            if buffer_size == None {
                buffer_size = Some(1024);
            }
            Base64Decoder {
                buffer: vec![0; 0],
                base64_buffer: vec![0; 0]
            }
        }
    }
}
