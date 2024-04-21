pub mod stream_step {
    use std::marker::{Send, Sync};
    use std::{io::Read, io::Write, os::fd::AsRawFd};
    use crate::{PipelineDirection, PipelineStep};

    pub struct StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        stream: T,
        pipeline_direction: PipelineDirection,
    }

    impl<T> StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        pub fn new(stream: T) -> Self {
            Self {
                stream: stream,
                pipeline_direction: PipelineDirection::Forward,
            }
        }
    }

    impl<T> PipelineStep for StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            self.pipeline_direction = direction
        }
        
        fn start(&self) {
            
        }
    }

    impl<T> Read for StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.stream.read(buf)
        }
    }

    impl<T> Write for StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.stream.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.stream.flush()
        }
    }
}
