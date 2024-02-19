#[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use clap::Arg;
    use std::{
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

    #[repr(u32)]
    #[derive(Copy, Clone, Eq, Debug)]
    pub enum PipelineStepType {
        Source = 0x01,
        Middle = 0x02,
        Destination = 0x04,
        SourceAndDest = 0x05,
    }

    impl BitAnd for PipelineStepType {
        type Output = PipelineStepType;

        fn bitand(self, rhs: Self) -> Self::Output {
            let result_u32 = (self as u32 & rhs as u32);
            match result_u32 {
                0x01 => PipelineStepType::Source,
                0x02 => PipelineStepType::Middle,
                0x04 => PipelineStepType::Destination,
                0x05 => PipelineStepType::SourceAndDest,
                _ => panic!("Invalid combination of flags"),
            }
        }
    }

    impl PartialEq for PipelineStepType {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    pub trait PipelineStep: Read + Write + Send + Sync {
        fn get_step_type(&self) -> PipelineStepType;
        fn len(&self) -> std::io::Result<usize> ;
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
        pub fn new(
            steps: Vec<Box<dyn PipelineStep>>,
            buffer_size: Option<usize>,
        ) -> Result<Self, IOError> {
            if steps.len() < 2 {
                Err(IOError::InvalidStep(format!(
                    "Step count must greater than two."
                )))
            } else if steps.first().unwrap().get_step_type() & PipelineStepType::Source
                != PipelineStepType::Source
            {
                Err(IOError::InvalidStep(format!(
                    "First step type must be PipelineStepType::Source."
                )))
            } else if steps.last().unwrap().get_step_type() & PipelineStepType::Destination
                != PipelineStepType::Destination
            {
                Err(IOError::InvalidStep(format!(
                    "Last step type must be PipelineStepType::Destination."
                )))
            } else {
                if steps.len() > 2 {
                    for i in 1..steps.len() - 2 {
                        if steps[i].get_step_type() & PipelineStepType::Middle
                            != PipelineStepType::Middle
                        {
                            return Err(IOError::InvalidStep(format!(
                                "Middle step type must be PipelineStepType::Middle."
                            )));
                        }
                    }
                }

                Ok(Pipeline {
                    steps,
                    buffer_size: Some(buffer_size.unwrap_or(1024)),
                })
            }
        }

        pub fn read_source(&mut self) -> Result<(), IOError> {
            for i in 0..self.steps.len() - 1 {
                let size = std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0;  size];
                let size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    self.steps[i + 1].write(&data[0..size]).unwrap();
                    self.steps[i + 1].flush().unwrap();
                }
            }
            Ok(())
        }

        pub fn read_destination(&mut self) -> Result<(), IOError> {
            for i in (1..self.steps.len()).rev() {
                let size = std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0;  size];
                let size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    self.steps[i - 1].write(&data[0..size]).unwrap();
                    self.steps[i - 1].flush().unwrap();
                }
            }
            Ok(())
        }
    }
}
