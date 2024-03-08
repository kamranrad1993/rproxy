pub mod stream_step {
    use std::marker::{Send, Sync};
    use std::{io::Read, io::Write, os::fd::AsRawFd};

    use crate::pipeline_module::pipeline::PipelineStepType;
    use crate::{PipelineDirection, PipelineStep};

    pub struct StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        stream: T,
        step_type: PipelineStepType,
        pipeline_direction: PipelineDirection,
    }

    impl<T> StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        pub fn new(stream: T, step_type: PipelineStepType) -> Self {
            Self {
                stream: stream,
                step_type: step_type,
                pipeline_direction: PipelineDirection::Forward,
            }
        }
    }

    impl<T> PipelineStep for StreamStep<T>
    where
        T: Read + Write + AsRawFd + Send + Sync,
    {
        fn get_step_type(&self) -> PipelineStepType {
            self.step_type
        }

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

        fn fork(&mut self) -> Result<Box<dyn PipelineStep>, ()> {
            Err(())
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
