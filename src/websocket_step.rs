#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_source {
    use crate::cmd::{Cmd, Error};
    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType};
    use clap::{Arg, ArgAction, Command};
    use std::os::fd::FromRawFd;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
    };
    use tungstenite::{accept, protocol::Role, Message, WebSocket};

    pub struct WebsocketSource {
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

    impl Cmd for WebsocketSource {
        fn get_cmd(command: clap::Command) -> Result<Command, Error> {
            Ok(command.arg(
                Arg::new("websocket(ws)")
                    .long("websocket")
                    .action(ArgAction::Append)
                    .required(false),
            ))
        }
    }

    impl PipelineStep for WebsocketSource {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Source
        }
    }

    impl Default for WebsocketSource {
        fn default() -> Self {
            Self {
                _tcp_server: TcpListener::bind("127.0.0.1:0").unwrap(),
                tcp_stream: unsafe{TcpStream::from_raw_fd(0)},
            }
        }
    }

    impl WebsocketSource {
        fn new(address: &str) -> Self {
            let server = TcpListener::bind(address).unwrap();
            let r = server.accept().unwrap().0;
            accept(r.try_clone().unwrap()).unwrap();
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

#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use clap::{Arg, ArgAction, Command};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::{AsRawFd, FromRawFd};
    use tungstenite::client::client_with_config;
    use tungstenite::handshake::client::Request;
    use tungstenite::protocol::Role;
    use tungstenite::{Message, WebSocket};

    use crate::cmd::{Cmd, Error};

    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType};

    pub struct WebsocketDestination {
        tcp_stream: TcpStream,
    }

    impl PipelineStep for WebsocketDestination {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Destination
        }
    }

    impl Default for WebsocketDestination {
        fn default() -> Self {
            Self {
                tcp_stream: unsafe{TcpStream::from_raw_fd(0)},
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

    impl Cmd for WebsocketDestination {
        fn get_cmd(command: clap::Command) -> Result<Command, Error> {
            Ok(command.arg(
                Arg::new("websocket_c(ws)")
                    .long("websocket_c")
                    .action(ArgAction::Append)
                    .required(false),
            ))
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
