pub mod io_step {
    use crate::{
        pipeline_module::pipeline::{PipelineDirection, PipelineStep},
        BoxedClone,
    };
    use std::io::{stdin, stdout, Read, Write};

    pub struct STDioStep {}

    impl PipelineStep for STDioStep {
        fn len(&mut self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 = unsafe { libc::ioctl(0, libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        #[allow(unused_variables)]
        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {}

        fn start(&mut self) {}
    }

    impl BoxedClone for STDioStep {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(STDioStep::new())
        }
    }

    impl Default for STDioStep {
        fn default() -> Self {
            Self {}
        }
    }

    impl Read for STDioStep {
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

    impl Write for STDioStep {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut io = stdout();
            io.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut io = stdout();
            io.flush()
        }
    }

    impl STDioStep {
        pub fn new() -> STDioStep {
            STDioStep {}
        }
    }
}
