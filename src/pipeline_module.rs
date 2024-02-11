#[allow(non_snake_case, unused_variables, dead_code)]

pub mod cmd {
    use clap::Command;

    #[derive(Debug)]
    pub enum Error {
        Err(String),
    }

    pub trait Cmd {
        fn get_cmd(command: Command) -> Result<Command, Error>;
    }
}

pub mod pipeline {
    use crate::cmd::Cmd;
    use bitflags::*;
    use clap::Arg;
    use std::{
        io::{self, Read, Write},
        ops::BitAnd,
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

    #[derive(Flags)]
    #[repr(u32)]
    pub enum PipelineStepType {
        Source = 1 << 0,
        Middle = 1 << 1,
        Destination = 1 << 2,
        Source_Destination = 1 << 0 | 1 << 2,
    }

    impl PartialEq for PipelineStepType {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    pub trait PipelineStep: Read + Write {
        fn get_step_type(&self) -> PipelineStepType;
    }

    pub struct Pipeline {
        steps: Vec<Box<dyn PipelineStep>>,
        buffer_size: Option<usize>,
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
            let mut data: Vec<u8> = vec![0; self.buffer_size.unwrap()];
            for i in 0..self.steps.len() - 1 {
                let size = self.steps[i].read(data.as_mut_slice()).unwrap();
                self.steps[i + 1].write(&data[0..size]).unwrap();
            }
            Ok(())
        }

        pub fn read_destination(&mut self) -> Result<(), IOError> {
            let mut data: Vec<u8> = vec![0; self.buffer_size.unwrap()];
            for i in self.steps.len() - 1..1 {
                let size = self.steps[i].read(data.as_mut_slice()).unwrap();
                self.steps[i - 1].write(&data[0..size]).unwrap();
            }
            Ok(())
        }
    }

    impl Cmd for Pipeline {
        fn get_cmd(command: clap::Command) -> Result<clap::Command, crate::cmd::Error> {
            Ok(command)
        }
    }
}
