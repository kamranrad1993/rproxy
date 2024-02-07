#[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use std::{
        io::{self, Read, Write},
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

    pub enum PipelineStepType {
        Source,
        Middle,
        Destination,
    }

    impl PartialEq for PipelineStepType {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    pub trait PipelineStep: Read + Write + 'static {
        fn get_step_type(&self) -> PipelineStepType;
    }

    pub struct Pipeline {
        steps: Vec<Box<dyn PipelineStep>>,
    }

    impl Pipeline {
        pub fn new(steps: Vec<Box<dyn PipelineStep>>) -> Result<Self, IOError> {
            if steps.len() < 2 {
                Err(IOError::InvalidStep(format!(
                    "Step count mus greater tha two."
                )))
            } else if steps.first().unwrap().get_step_type() != PipelineStepType::Source {
                Err(IOError::InvalidStep(format!(
                    "First step typemust be PipelineStepType::Source."
                )))
            } else if steps.last().unwrap().get_step_type() != PipelineStepType::Destination {
                Err(IOError::InvalidStep(format!(
                    "last step typemust be PipelineStepType::Destination."
                )))
            } else {
                Ok(Pipeline { steps })
            }
        }

        pub fn run(&mut self) -> Result<(), IOError> {
            let mut data: Vec<u8> = vec![0; 1024];
            for i in 0..self.steps.len() - 2 {
                self.steps[i].read(data.as_mut_slice()).unwrap();
                self.steps[i + 1].write(data.as_mut_slice()).unwrap();
            }
            Ok(())
        }
    }
}

pub mod middle_ware {}

pub mod source {
    use super::pipeline::{ PipelineStep, PipelineStepType};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
    };
    use tungstenite::{accept, protocol::Role, Message, WebSocket};

    pub(crate) struct WebsocketSource {
        _tcp_server: TcpListener,
        tcp_stream: TcpStream,
    }

    impl Write for WebsocketSource {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            self.get_websocket().write(msg).unwrap();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
        }
    }

    impl Read for WebsocketSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                self.tcp_stream.read(buf)
            }
        }
    }

    impl PipelineStep for WebsocketSource {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Source
        }
    }

    impl WebsocketSource {
        fn new(address: &str) -> Self {
            let server = TcpListener::bind(address).unwrap();
            let r = server.accept().unwrap().0;
            let websocket = accept(r.try_clone().unwrap()).unwrap();
            WebsocketSource {
                _tcp_server: server,
                tcp_stream: r, // websocket: websocket,
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Server, None)
        }
    }
}

pub mod destination {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use tungstenite::client::client_with_config;
    use tungstenite::handshake::client::Request;
    use tungstenite::protocol::Role;
    use tungstenite::{Message, WebSocket};

    use super::pipeline::{PipelineStep, PipelineStepType};

    pub(crate) struct WebsocketDestination {
        tcp_stream: TcpStream,
    }

    impl PipelineStep for WebsocketDestination {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Destination
        }
    }

    impl Read for WebsocketDestination {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                self.tcp_stream.read(buf)
            }
        }
    }

    impl Write for WebsocketDestination {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            self.get_websocket().write(msg).unwrap();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
        }
    }

    impl WebsocketDestination {
        fn new(address: &str) -> Self {
            let connection = TcpStream::connect(address).unwrap();
            let req = Request::builder().uri(address).body(()).unwrap();
            let l = client_with_config(req, connection.try_clone().unwrap(), None).unwrap();
            //handle errors
            WebsocketDestination {
                tcp_stream: connection,
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Client, None)
        }
    }
}
