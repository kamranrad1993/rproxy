#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_source {
    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep};
    use crate::StreamStep;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
    };
    use tungstenite::http::Uri;
    use tungstenite::{accept, protocol::Role, Message, WebSocket};

    pub struct WebsocketSource {
        _tcp_server: TcpListener,
        tcp_stream: TcpStream,
        address: String,
    }

    impl Write for WebsocketSource {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            // let vec = Vec::from(buf);
            // let msg = Message::Binary(vec);
            // self.get_websocket().send(msg).unwrap();
            // Ok(buf.len())
            Ok(0)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            // self.get_websocket().flush().unwrap();
            // // self.tcp_stream.flush()
            Ok(())
        }
    }

    impl Read for WebsocketSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            // let mut available: usize = 0;
            // let result: i32 =
            //     unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };

            // if result == -1 {
            //     let errno = std::io::Error::last_os_error();
            //     Err(errno)
            // } else if available == 0 {
            //     Ok(0)
            // } else {
            //     // self.tcp_stream.read(buf)
            //     let m = &mut self.get_websocket().read().unwrap();
            //     // let mut m = &mut self
            //     //     .context
            //     //     .read::<TcpStream>(&mut self.tcp_stream)
            //     //     .unwrap();
            //     match m {
            //         Message::Text(data) => {
            //             unsafe {
            //                 let length = std::cmp::min(data.as_bytes().len(), buf.len());
            //                 std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), length);
            //                 Ok(length)
            //             }
            //             // buf.copy_from_slice(data.as_bytes());
            //         }
            //         Message::Binary(data) => {
            //             unsafe {
            //                 let length = std::cmp::min(data.len(), buf.len());
            //                 std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), length);
            //                 Ok(length)
            //             }
            //             // buf.copy_from_slice(data.as_slice());
            //             // buf = data.as_mut_slice();
            //         }
            //         Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
            //             Ok(0)
            //         }
            //     }
            // }
            Ok(0)
        }
    }

    impl PipelineStep for WebsocketSource {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            // println!("{}", direction);
        }

        fn start(&self) {
            
        }
    }

    impl WebsocketSource {
        pub fn new(address: &str) -> Self {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let server = TcpListener::bind(addr.clone()).unwrap();
            let r = server.accept().unwrap().0;
            accept(r.try_clone().unwrap()).unwrap();
            WebsocketSource {
                _tcp_server: server,
                tcp_stream: r,
                address: addr.clone(),
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Server, None)
        }
    }
}
#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::{Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep};

    pub struct WebsocketDestination {
        tcp_stream: TcpStream,
        context: WebSocketContext,
    }

    impl PipelineStep for WebsocketDestination {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            // println!("{}", direction);
        }

        fn start(&self) {
            
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
                    Message::Text(data) => unsafe {
                        let length = std::cmp::min(data.as_bytes().len(), buf.len());
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.as_bytes().len());
                        Ok(length)
                    },
                    Message::Binary(data) => unsafe {
                        let length = std::cmp::min(data.len(), buf.len());
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                        Ok(length)
                    },
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

    #[allow(unreachable_code)]
    impl WebsocketDestination {
        pub fn new(address: &str) -> Self {
            let mut connection: Option<TcpStream> = None;

            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            let mut port = 0;
            if uri.port() != None {
                port = uri.port().unwrap().as_u16();
            } else {
                port = match uri.scheme_str() {
                    Some("ws") => 80,
                    Some("wss") => 443,
                    Some("http") => 80,
                    Some("https") => 443,
                    None | _ => {
                        panic!("unknow uri scheme")
                    }
                };
            }

            addr.push_str(":");
            addr.push_str(port.to_string().as_str());
            let connection = TcpStream::connect(addr).unwrap();
            let req: tungstenite::http::Request<()> = uri.into_client_request().unwrap();
            let l = client(req, connection.try_clone().unwrap()).unwrap();

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

pub mod wss_destination {
    use openssl::ssl::{SslConnector, SslConnectorBuilder, SslMethod, SslStream, SslVerifyMode};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::{HeaderName, Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::PipelineStep;

    pub struct WssDestination {
        tcp_stream: TcpStream,
        ssl_stream: WebSocket<SslStream<TcpStream>>,
        context: WebSocketContext,
    }

    impl WssDestination {
        pub fn new(address: &str) -> WssDestination {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            let mut port = 0;
            if uri.port() != None {
                port = uri.port().unwrap().as_u16();
            } else {
                port = match uri.scheme_str() {
                    Some("ws") => 80,
                    Some("wss") => 443,
                    Some("http") => 80,
                    Some("https") => 443,
                    None | _ => {
                        panic!("unknow uri scheme")
                    }
                };
            }

            addr.push_str(":");
            addr.push_str(port.to_string().as_str());
            let connection = TcpStream::connect(addr.clone()).unwrap();

            let mut ssl_connector_builder: SslConnectorBuilder =
                SslConnector::ConnectConfigurationbuilder(SslMethod::tls()).unwrap();
            // ssl_connector_builder.set_verify(SslVerifyMode::NONE);
            // ssl_connector_builder.set_verify_callback(SslVerifyMode::NONE, |r, context|{
            //     true
            // });
            // ssl_connector_builder.set_verify_callback(SslVerifyMode::PEER, |r, context|{
            //     true
            // });

            let mut ssl_connector = ssl_connector_builder.build();
            let ssl_connection = ssl_connector
                .configure()
                .unwrap()
                // .verify_hostname(false)
                // .use_server_name_indication(false)
                .connect(uri.host().unwrap(), connection.try_clone().unwrap())
                .unwrap();

            let req: tungstenite::http::Request<()> = uri.into_client_request().unwrap();
            // let r = ssl_connection.get_mut();
            let (socket, _response) = client(req, ssl_connection).unwrap();

            Self {
                ssl_stream: socket,
                tcp_stream: connection,
                context: WebSocketContext::new(Role::Client, None),
            }
        }

        pub fn get_websocket(&mut self) -> WebSocket<&mut SslStream<TcpStream>> {
            WebSocket::from_raw_socket(self.ssl_stream.get_mut(), Role::Client, None)
        }
    }

    impl PipelineStep for WssDestination {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: crate::PipelineDirection) {}

        fn start(&self) {
            
        }
    }

    impl Read for WssDestination {
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
                    Message::Text(data) => unsafe {
                        let length = std::cmp::min(data.as_bytes().len(), buf.len());
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.as_bytes().len());
                        Ok(length)
                    },
                    Message::Binary(data) => unsafe {
                        let length = std::cmp::min(data.len(), buf.len());
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                        Ok(length)
                    },
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                        Ok(0)
                    }
                }
                // self.tcp_stream.read(buf)
            }
        }
    }

    impl Write for WssDestination {
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
}
