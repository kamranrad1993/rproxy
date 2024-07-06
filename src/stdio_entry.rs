pub mod io_entry {
    use crate::{
        pipeline_module::pipeline::{PipelineDirection, PipelineStep},
        BoxedClone, Entry, IOError, Pipeline,
    };
    use std::{
        io::{stdin, stdout, Read, Write},
        os::fd::AsRawFd,
        thread,
        time::Duration,
    };

    pub struct STDioEntry {
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Entry for STDioEntry {
        fn len(stream: &mut dyn AsRawFd) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn new(config: String, pipeline: crate::Pipeline, loop_time: u64) -> Self {
            STDioEntry {
                pipeline: pipeline,
                loop_time: loop_time,
            }
        }

        fn listen(&mut self) {
            self.pipeline.start();
            loop {
                let len = STDioEntry::len(&mut std::io::stdin()).unwrap();
                if len > 0 {
                    let mut buf: Vec<u8> = vec![0; len];
                    self.read(buf.as_mut_slice()).unwrap();
                    match self.pipeline.write(buf) {
                        Ok(size) => {}
                        Err(e) => match e {
                            IOError::InvalidConnection
                            | IOError::InvalidBindAddress
                            | IOError::UnknownError(_)
                            | IOError::IoError(_)
                            | IOError::ParseError
                            | IOError::InvalidStep(_)
                            | IOError::InvalidData(_)
                            | IOError::Error(_) => {
                                println!("{}", e);
                                break;
                            }
                            IOError::EmptyData => {}
                        },
                    }
                }

                if self.pipeline.read_available() {
                    match &mut self.pipeline.read() {
                        Ok(data) => {
                            if data.len() > 0 {
                                self.write(data.as_mut_slice()).unwrap();
                                self.flush().unwrap();
                            }
                        }
                        Err(e) => match e {
                            IOError::InvalidConnection
                            | IOError::InvalidBindAddress
                            | IOError::UnknownError(_)
                            | IOError::IoError(_)
                            | IOError::ParseError
                            | IOError::InvalidStep(_)
                            | IOError::InvalidData(_)
                            | IOError::Error(_) => {
                                println!("{}", e);
                                break;
                            }
                            IOError::EmptyData => {}
                        },
                    }
                }
                thread::sleep(Duration::from_millis(self.loop_time));
            }
        }
    }

    impl Clone for STDioEntry {
        fn clone(&self) -> STDioEntry {
            STDioEntry {
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time,
            }
        }
    }

    impl Read for STDioEntry {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut io = stdin();
            let mut available: usize = 0;
            let result: i32 = unsafe { libc::ioctl(0, libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else if available == 0 {
                Ok(0)
            } else {
                io.read(buf)
            }
        }
    }

    impl Write for STDioEntry {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut io = stdout();
            io.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut io = stdout();
            io.flush()
        }
    }
}
