// #[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use std::{
        fmt::Display,
        io::{self, Read, Write},
        ops::{BitAnd, Deref, DerefMut},
        string::ParseError,
    };

    #[derive(Debug)]
    pub enum IOError {
        InvalidConnection,
        InvalidBindAddress,
        UnknownError(String),
        IoError(io::Error),
        ParseError(ParseError),
        InvalidStep(String),
    }

    pub enum PipelineDirection {
        Forward = 0x01,
        Backward = 0x02,
    }

    impl Clone for PipelineDirection {
        fn clone(&self) -> Self {
            match self {
                Self::Forward => Self::Forward,
                Self::Backward => Self::Backward,
            }
        }
    }

    impl Copy for PipelineDirection {}

    impl PartialEq for PipelineDirection {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    impl Display for PipelineDirection {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PipelineDirection::Forward => write!(f, "PipelineDirection::Forward"),
                PipelineDirection::Backward => write!(f, "PipelineDirection::Backward"),
            }
        }
    }

    pub trait PipelineStep: Read + Write + Send + Sync {
        fn start(&self);
        fn len(&self) -> std::io::Result<usize>;
        fn set_pipeline_direction(&mut self, direction: PipelineDirection);
    }

    pub struct Pipeline {
        steps: Vec<Box<dyn PipelineStep>>,
        buffer_size: Option<usize>,
    }

    impl Deref for Pipeline {
        type Target = [Box<dyn PipelineStep>];

        fn deref(&self) -> &Self::Target {
            &self.steps
        }
    }

    impl DerefMut for Pipeline {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.steps
        }
    }

    impl Pipeline {
        pub fn new(steps: Vec<Box<dyn PipelineStep>>, buffer_size: Option<usize>) -> Self {
            Pipeline {
                steps: steps,
                buffer_size: Some(buffer_size.unwrap_or(1024)),
            }
        }

        pub fn read_source(&mut self) -> Result<(), IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Forward);
            }

            for i in 0..self.steps.len() - 1 {
                let mut size =
                    std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0; size];
                // self.steps[i].set_pipeline_direction(PipelineDirection::Forward);
                size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    // self.steps[i + 1].set_pipeline_direction(PipelineDirection::Forward);
                    self.steps[i + 1].write(&data[0..size]).unwrap();
                    self.steps[i + 1].flush().unwrap();
                }
            }
            Ok(())
        }

        pub fn read_destination(&mut self) -> Result<(), IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Backward);
            }

            for i in (1..self.steps.len()).rev() {
                let mut size =
                    std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0; size];
                // self.steps[i].set_pipeline_direction(PipelineDirection::Backward);
                size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    // self.steps[i - 1].set_pipeline_direction(PipelineDirection::Backward);
                    self.steps[i - 1].write(&data[0..size]).unwrap();
                    self.steps[i - 1].flush().unwrap();
                }
            }
            Ok(())
        }
    }
}
