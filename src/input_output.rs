#[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use async_trait::async_trait;
    use std::{
        io::{self, Read, Write},
        string::ParseError,
        collections::HashMap
    };

    use super::destination::WebsocketDestination;
    use crate::source::WebsocketSource;

    #[derive(Debug)]
    pub enum IOError {
        InvalidConnection,
        InvalidBindAddress,
        UnknownError(String),
        IoError(io::Error),
        ParseError(ParseError),
    }

    pub enum SourceProtocols{
        Websocket(WebsocketSource),
    }

    pub enum DestinationProtocols{
        Websocket(WebsocketDestination),
    }




    pub trait ProtocolName {
        const VALUE: &'static str;
        fn get_protocol() -> &'static str;
    }

    #[async_trait]
    pub trait PipelineStep: ProtocolName + Read + Write {
        fn new(address: &str) -> Self;
        fn validate_address(address: String) -> Result<String, IOError>;
    }

    pub struct Pipeline<T: PipelineStep> {
        source: T,
        destination: T,
    }

    impl<T: PipelineStep> Pipeline<T> {
        pub fn new(source: String, destination: String) -> Pipeline<T> {
            
        }
    }
}

pub mod middle_ware {}

pub mod source {
    use super::pipeline::{IOError, PipelineStep, ProtocolName};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
        string::ParseError,
    };
    use tungstenite::{accept, protocol::Role, Message, WebSocket};

    pub(crate) struct WebsocketSource {
        _tcp_server: TcpListener,
        tcp_stream: TcpStream,
    }

    impl ProtocolName for WebsocketSource {
        const VALUE: &'static str = "ws";

        fn get_protocol() -> &'static str {
            WebsocketSource::VALUE
        }
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
        fn new(address: &str) -> Self {
            let server = TcpListener::bind(address).unwrap();
            let r = server.accept().unwrap().0;
            let websocket = accept(r.try_clone().unwrap()).unwrap();
            WebsocketSource {
                _tcp_server: server,
                tcp_stream: r, // websocket: websocket,
            }
        }

        fn validate_address(address: String) -> Result<String, IOError> {
            let parsed_address: Vec<&str> = address.split(':').collect();
            if parsed_address[0] == WebsocketSource::get_protocol() {
                Ok(String::from(parsed_address[0]))
            } else {
                Err(IOError::UnknownError(format!(
                    "Failed to parse '{}' as protocol.",
                    parsed_address[0]
                )))
            }
        }
    }

    impl WebsocketSource {
        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Server, None)
        }
    }
}

pub mod destination {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::os::fd::AsRawFd;
    use tungstenite::client::client_with_config;
    use tungstenite::handshake::client::Request;
    use tungstenite::protocol::Role;
    use tungstenite::{Message, WebSocket};

    use super::pipeline::{IOError, PipelineStep, ProtocolName};

    pub(crate) struct WebsocketDestination {
        tcp_stream: TcpStream,
    }

    impl ProtocolName for WebsocketDestination {
        const VALUE: &'static str = "ws";

        fn get_protocol() -> &'static str {
            WebsocketDestination::VALUE
        }
    }

    impl PipelineStep for WebsocketDestination {
        fn new(address: &str) -> Self {
            let connection = TcpStream::connect(address).unwrap();
            let req = Request::builder().uri(address).body(()).unwrap();
            let l = client_with_config(req, connection.try_clone().unwrap(), None).unwrap();
            //handle errors
            WebsocketDestination {
                tcp_stream: connection,
            }
        }

        fn validate_address(address: String) -> Result<String, IOError> {
            let parsed_address: Vec<&str> = address.split(':').collect();
            if parsed_address[0] == WebsocketDestination::get_protocol() {
                Ok(String::from(parsed_address[0]))
            } else {
                Err(IOError::UnknownError(format!(
                    "Failed to parse '{}' as protocol.",
                    parsed_address[0]
                )))
            }
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
        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Client, None)
        }
    }
}
