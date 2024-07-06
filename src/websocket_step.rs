#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use bytes::BytesMut;
    use http::{response, StatusCode, Version};
    use hyper::{body::Body, Method, Request, Response, Uri};
    use openssl::error;
    use polling::{Event, Events, Poller};
    use std::fmt::{Display, Error};
    use std::io::{self, Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::{self, FromStr};
    use std::time::Duration;
    use tokio_util::codec::{Decoder, Encoder};
    use websocket_codec::{Message, MessageCodec};

    use crate::pipeline_module::pipeline::{IOError, PipelineDirection, PipelineStep};
    use crate::{
        get_available_bytes, http_tools, read_response, write_request, BoxedClone,
        WssDestination,
    };

    pub struct WebsocketDestination {
        tcp_stream: Option<TcpStream>,
        address: String,
    }

    impl PipelineStep for WebsocketDestination {
        fn len(&mut self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 = unsafe {
                libc::ioctl(
                    self.tcp_stream.as_ref().unwrap().as_raw_fd(),
                    libc::FIONREAD,
                    &mut available,
                )
            };
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

        fn start(&mut self) {
            let uri: Uri = self.address.parse::<Uri>().unwrap();
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
            let mut connection = TcpStream::connect(&addr).unwrap();
            connection.set_nonblocking(false).unwrap();

            WebsocketDestination::handshake(&mut connection, addr).unwrap();

            self.tcp_stream = Some(connection);
        }
    }

    impl BoxedClone for WebsocketDestination {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(WebsocketDestination::new(&self.address))
        }
    }

    impl crate::Read for WebsocketDestination {
        fn read(&mut self) -> Result<Vec<u8>, IOError> {
            let mut available: usize = 0;
            let result: i32 = unsafe {
                libc::ioctl(
                    self.get_stream().as_raw_fd(),
                    libc::FIONREAD,
                    &mut available,
                )
            };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(IOError::IoError(errno))
            } else if available == 0 {
                Err(IOError::EmptyData)
            } else {
                let mut byteData = BytesMut::new();
                byteData.resize(available, 0);
                if let Err(e) = self.get_stream().read(byteData.as_mut()) {
                    return Err(IOError::IoError(e));
                }

                match MessageCodec::client().decode(&mut byteData) {
                    Ok(msg) => match msg {
                        Some(msg) => match msg.opcode() {
                            websocket_codec::Opcode::Text | websocket_codec::Opcode::Binary => {
                                return Ok(msg.data().to_vec());
                            }
                            websocket_codec::Opcode::Close => {
                                let e = io::Error::new(
                                    io::ErrorKind::ConnectionAborted,
                                    "server disconnected",
                                );
                                return Err(IOError::IoError(e));
                            }
                            websocket_codec::Opcode::Ping | websocket_codec::Opcode::Pong => {
                                Err(IOError::EmptyData)
                            }
                        },
                        None => {
                            let e = io::Error::new(
                                io::ErrorKind::InvalidData,
                                "no valid message found",
                            );
                            return Err(IOError::IoError(e));
                        }
                    },
                    Err(e) => {
                        let e = format!("{}", e);
                        let e = io::Error::new(io::ErrorKind::Other, e);
                        return Err(IOError::IoError(e));
                    }
                }
            }
        }
    }

    impl Write for WebsocketDestination {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec_buf = Vec::from(buf);
            let msg = Message::binary(vec_buf);
            let mut bytebuf: BytesMut = BytesMut::new();
            MessageCodec::client().encode(&msg, &mut bytebuf).unwrap();

            match self.get_stream().write(bytebuf.to_vec().as_slice()) {
                Ok(size) => {
                    if let Err(e) = self.get_stream().flush() {
                        return Err(e);
                    }
                    return Ok(size);
                }
                Err(e) => return Err(e),
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_stream().flush()
        }
    }

    #[allow(unreachable_code)]
    impl WebsocketDestination {
        pub fn new(address: &str) -> Self {
            WebsocketDestination {
                tcp_stream: None,
                address: String::from_str(address).unwrap(),
            }
        }

        fn handshake(mut stream: &mut TcpStream, address: String) -> Result<(), IOError> {
            //send request
            let mut rand_buf = [0u8; 16];
            openssl::rand::rand_bytes(&mut rand_buf).unwrap();
            let sec_websocket_key = base64::encode(rand_buf);

            let request: Request<Vec<u8>> = Request::builder()
                .method(Method::GET)
                .uri("/")
                .header("Host", address)
                .header("Accept", "text/html; charset=utf-8")
                .header("Keep-Alive","timeout=6553600")
                .header("User-Agent", "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0")
                .header("Upgrade", "websocket")
                .header("Connection", "Upgrade")
                .header("Sec-WebSocket-Key", sec_websocket_key)
                .header("Sec-WebSocket-Version", "13")
                .header("Upgrade-Insecure-Requests", "1")
                .body(vec![0; 0])
                .unwrap();

            write_request(&mut stream, &request)?;
            
            let res = read_response(stream)?;

            if res.status() != StatusCode::from_u16(101).unwrap() {
                println!(
                    "response: {}",
                    std::str::from_utf8(res.body()).unwrap_or("")
                );
                let msg : String = std::str::from_utf8(res.body()).unwrap_or("").to_string();
                return Err(IOError::InvalidData(msg));
            }

            Ok(())
        }

        fn get_stream(&self) -> &TcpStream {
            self.tcp_stream.as_ref().unwrap()
        }
    }
}

pub mod wss_destination {
    use core::panic;
    use openssl::ssl::{SslConnector, SslConnectorBuilder, SslMethod, SslStream};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::FromStr;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::{HeaderName, Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::{BoxedClone, IOError, PipelineStep};

    pub struct WssDestination {
        tcp_stream: TcpStream,
        ssl_stream: WebSocket<SslStream<TcpStream>>,
        context: WebSocketContext,
        address: String,
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

            #[cfg(feature = "has_not_builder")]
            let mut ssl_connector_builder: SslConnectorBuilder =
                SslConnector::ConnectConfigurationbuilder(SslMethod::tls()).unwrap();
            #[cfg(feature = "has_builder")]
            let mut ssl_connector_builder: SslConnectorBuilder =
                SslConnector::builder(SslMethod::tls()).unwrap();

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
                address: String::from_str(address).unwrap(),
            }
        }

        pub fn get_websocket(&mut self) -> WebSocket<&mut SslStream<TcpStream>> {
            WebSocket::from_raw_socket(self.ssl_stream.get_mut(), Role::Client, None)
        }
    }

    impl PipelineStep for WssDestination {
        fn len(&mut self) -> std::io::Result<usize> {
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

        fn start(&mut self) {}
    }

    impl BoxedClone for WssDestination {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(WssDestination::new(&self.address))
        }
    }

    impl crate::Read for WssDestination {
        fn read(&mut self) -> Result<Vec<u8>, IOError> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(IOError::IoError(errno))
            } else if available == 0 {
                Err(IOError::EmptyData)
            } else {
                let m = &mut self.get_websocket().read().unwrap();
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match m {
                    Message::Text(data) => unsafe { Ok(data.as_bytes().to_vec()) },
                    Message::Binary(data) => unsafe { Ok(data.clone()) },
                    Message::Close(e) => {
                        let errno = std::io::Error::last_os_error();
                        Err(IOError::IoError(errno))
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                        Err(IOError::EmptyData)
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
