// #[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use std::{
        fmt::Display,
        io::{self, Write},
        ops::{BitAnd, Deref, DerefMut},
        string::ParseError,
    };

    use strum::Display;

    #[derive(Debug, Display)]
    pub enum IOError {
        InvalidConnection,
        InvalidBindAddress,
        UnknownError(String),
        IoError(io::Error),
        ParseError,
        InvalidStep(String),
        InvalidData(String),
        EmptyData,
        Error(Box<dyn std::error::Error + Send + Sync>)
    }

    impl From<std::io::Error>  for IOError{
        fn from(value: std::io::Error) -> Self {
            IOError::IoError(value)
        }
    }

    impl From<Box<dyn std::error::Error + Send + Sync>> for IOError {
        fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
            IOError::Error(value)
        }
    }

    pub trait Read {
        fn read(&mut self) -> Result<Vec<u8>, IOError>;
    }

    pub trait BoxedClone {
        fn bclone(&self) -> Box<dyn PipelineStep>;
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

    pub trait PipelineStep: Read + Write + Send + Sync + BoxedClone {
        fn start(&mut self);
        fn len(&mut self) -> std::io::Result<usize>;
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

    impl Clone for Pipeline {
        fn clone(&self) -> Self {
            let mut steps = Vec::<Box<dyn PipelineStep>>::new();
            for step in self.steps.as_slice() {
                steps.push(step.bclone())
            }
            Pipeline::new(steps, self.buffer_size)
        }
    }

    impl Pipeline {
        pub fn new(steps: Vec<Box<dyn PipelineStep>>, buffer_size: Option<usize>) -> Self {
            Pipeline {
                steps: steps,
                buffer_size: Some(buffer_size.unwrap_or(1024)),
            }
        }

        pub fn start(&mut self) {
            for i in 0..self.steps.len() {
                self.steps[i].as_mut().start();
            }
        }

        pub fn write(&mut self, mut data: Vec<u8>) -> Result<usize, IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Forward);
            }

            for i in 0..self.steps.len() {
                self.steps[i].write(&data).unwrap();
                self.steps[i].flush().unwrap();
                if i != (self.steps.len() - 1) {
                    data = self.steps[i].read()?;
                }
            }
            Ok(data.len())
        }

        pub fn read(&mut self) -> Result<Vec<u8>, IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Backward);
            }

            let mut data: Vec<u8> = Vec::new();
            data.resize(self.buffer_size.unwrap(), 0);

            for i in (0..self.steps.len()).rev() {
                data = self.steps[i].read()?;
                if data.len() > 0 && i != 0 {
                    self.steps[i - 1].write(&data).unwrap();
                    self.steps[i - 1].flush().unwrap();
                }
            }
            Ok(data)
        }

        pub fn read_available(&mut self) -> bool {
            self.steps.last_mut().unwrap().len().unwrap() != 0
        }

        pub fn len(&mut self) -> std::io::Result<usize>{
            self.steps.last_mut().unwrap().len()
        }
    }
}
