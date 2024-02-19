#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_source {
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
            } else if available == 0 {
                Ok(0)
            } else {
                self.tcp_stream.read(buf)
            }
        }
    }

    impl PipelineStep for WebsocketSource {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Source
        }
        
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            }else {
                Ok(available)
            }
        }
    }

    impl WebsocketSource {
        pub fn new(address: &str) -> Self {
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
    use tungstenite::client::client;
    use tungstenite::client::{client_with_config, IntoClientRequest};
    use tungstenite::handshake::client::Request;
    use tungstenite::http::{header, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{Message, WebSocket};

    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType};

    pub struct WebsocketDestination {
        tcp_stream: TcpStream,
        context: WebSocketContext,
    }

    impl PipelineStep for WebsocketDestination {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Destination
        }

        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            }else {
                Ok(available)
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
            } else if available == 0 {
                Ok(0)
            } else {
                let m = &mut self.get_websocket().read().unwrap();
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match m {
                    Message::Text(data) => {
                        unsafe {
                            std::ptr::copy(
                                data.as_mut_ptr(),
                                buf.as_mut_ptr(),
                                data.as_bytes().len(),
                            );
                        }
                        Ok(data.as_bytes().len())
                    }
                    Message::Binary(data) => {
                        unsafe {
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                        }
                        Ok(data.len())
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                        Ok(0)
                    }
                }
                // self.tcp_stream.read(buf)
            }
        }
    }

    impl Write for WebsocketDestination {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            let result = self.get_websocket().send(msg);
            match result {
                Ok(_) => Ok(buf.len()),
                Err(error) => {
                    panic!("{}", error);
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
        }
    }

    impl WebsocketDestination {
        pub fn new(address: &str) -> Self {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let connection = TcpStream::connect(addr).unwrap();
            // let req = Request::builder().uri(address).body(()).unwrap();
            let req: tungstenite::http::Request<()> = uri.into_client_request().unwrap();
            // let l = client_with_config(req, connection.try_clone().unwrap(), None).unwrap();
            let mut l = client(req, connection.try_clone().unwrap()).unwrap();

            //handle errors
            WebsocketDestination {
                tcp_stream: connection,
                context: WebSocketContext::new(Role::Client, None),
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Client, None)
        }
    }
}
