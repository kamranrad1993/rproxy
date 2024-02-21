pub mod io_step {
    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType, PipelineDirection};
    use clap::{Arg, ArgAction};
    use std::io::{stdin, stdout, Read, Write};
    use std::os::fd::AsFd;

    pub struct STDioStep {}

    impl PipelineStep for STDioStep {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::SourceAndDest
        }

        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(0, libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            }else {
                Ok(available)
            }
        }

        fn set_pipeline_direction (&mut self, direction: PipelineDirection){
            
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
