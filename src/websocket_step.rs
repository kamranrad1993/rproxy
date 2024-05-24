#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use bytes::BytesMut;
    use http::response;
    use httparse::{Response as HttpResponse, EMPTY_HEADER};
    use std::fmt::{Display, Error};
    use std::io::{self, Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::{self, FromStr};
    use tokio_util::codec::{Decoder, Encoder};
    use websocket_codec::{Message, MessageCodec};
    use hyper::{Request, Response, Method, Uri, body::Body};

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep};
    use crate::{BoxedClone, WssDestination};

    pub struct WebsocketDestination {
        tcp_stream: TcpStream,
        address: String,
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

        fn start(&self) {}
    }

    impl BoxedClone for WebsocketDestination {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(WebsocketDestination::new(&self.address))
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
                let mut byteData = BytesMut::new();
                byteData.resize(available, 0);
                if let Err(e)  = self.tcp_stream.read(byteData.to_vec().as_mut_slice()){
                    return Err(e)
                }

                match MessageCodec::client().decode(&mut byteData) {
                    Ok(msg) => match msg {
                        Some(msg) => {
                            match msg.opcode() {
                                websocket_codec::Opcode::Text|
                                websocket_codec::Opcode::Binary => {
                                    unsafe {
                                        std::ptr::copy(
                                            msg.data().as_ptr(),
                                            buf.as_mut_ptr(),
                                            msg.data().len(),
                                        );
                                    }
                                    return Ok(msg.data().len());
                                },
                                websocket_codec::Opcode::Close => {
                                    let e = io::Error::new(io::ErrorKind::ConnectionAborted, "server disconnected");
                                    return Err(e)
                                },
                                websocket_codec::Opcode::Ping |
                                websocket_codec::Opcode::Pong => {
                                    Ok(0)
                                }
                            }
                            
                        }
                        None => {
                            let e = io::Error::new(
                                io::ErrorKind::InvalidData,
                                "no valid message found",
                            );
                            return Err(e);
                        }
                    },
                    Err(e) => {
                        let e = format!("{}", e);
                        let e = io::Error::new(io::ErrorKind::Other, e);
                        return Err(e);
                    }
                }
                // let m = &mut self.get_websocket().read().unwrap();
                // match m {
                //     Message::Text(data) => unsafe {
                //         std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.as_bytes().len());
                //         Ok(data.as_bytes().len())
                //     },
                //     Message::Binary(data) => unsafe {
                //         std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                //         Ok(data.len())
                //     },
                //     Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                //         Ok(0)
                //     }
                // }
            }
        }
    }

    impl Write for WebsocketDestination {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec_buf = Vec::from(buf);
            let msg = Message::binary(vec_buf);
            let mut bytebuf: BytesMut = BytesMut::new();
            MessageCodec::client().encode(&msg, &mut bytebuf).unwrap();

            match self.tcp_stream.write(bytebuf.to_vec().as_slice()) {
                Ok(size) => {
                    if let Err(e) = self.tcp_stream.flush() {
                        return Err(e);
                    }
                    return Ok(size);
                }
                Err(e) => return Err(e),
            }

            // let vec = Vec::from(buf);
            // let msg = Message::Binary(vec);
            // let result = self.get_websocket().send(msg);
            // match result {
            //     Ok(_) => Ok(buf.len()),
            //     Err(error) => {
            //         panic!("{}", error);
            //     }
            // }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.tcp_stream.flush()
        }
    }

    #[allow(unreachable_code)]
    impl WebsocketDestination {
        pub fn new(address: &str) -> Self {
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
            let connection = TcpStream::connect(&addr).unwrap();
            // let req: tungstenite::http::Request<()> = uri.into_client_request().unwrap();
            // let l = client(req, connection.try_clone().unwrap()).unwrap();

            WebsocketDestination::handshake(&connection, addr);

            //handle errors
            WebsocketDestination {
                tcp_stream: connection,
                // context: WebSocketContext::new(Role::Client, None),
                address: String::from_str(address).unwrap(),
            }
        }

        fn serialize_request<T>(request: &Request<T>) -> Vec<u8>
        where
            T: Display,
        {
            let mut buffer = Vec::new();

            // Serialize the request line
            write!(
                buffer,
                "{} {} HTTP/1.1\r\n",
                request.method(),
                request.uri()
            )
            .unwrap();

            // Serialize the headers
            for (key, value) in request.headers() {
                write!(buffer, "{}: {}\r\n", key, value.to_str().unwrap()).unwrap();
            }

            // End of headers
            write!(buffer, "\r\n").unwrap();

            // Serialize the body
            write!(buffer, "{}", request.body()).unwrap();

            buffer
        }

        fn handshake(mut stream: &TcpStream, address: String) -> std::io::Result<()> {

            let mut headers = [EMPTY_HEADER; 16];
            
            let l = httparse::Request::new(&mut headers);
            
            //send request
            let mut rand_buf = [0u8; 16];
            openssl::rand::rand_bytes(&mut rand_buf).unwrap();
            let sec_websocket_key = base64::encode(rand_buf);

            // Parse the HTTP request
            let request: Request<&str> = Request::builder()
                .method(Method::GET)
                .uri(&address)
                .header("Host", address)
                .header("Upgrade", "websocket")
                .header("Connection", "Upgrade")
                .header("Sec-WebSocket-Key", sec_websocket_key)
                .header("Sec-WebSocket-Version", "13")
                .body("")
                .unwrap();

            let l = WebsocketDestination::serialize_request(&request);
            stream
                .write_all(&WebsocketDestination::serialize_request(&request))
                .unwrap();
            stream.flush().unwrap();

            //read response
            let mut buffer = [0; 1024];
            let read_size = stream.read(&mut buffer).unwrap();

            let mut headers = [EMPTY_HEADER; 16];
            let mut res = HttpResponse::new(&mut headers);
            match res.parse(&buffer[0..read_size]) {
                Ok(status) => {
                    // println!("{}", res.code.unwrap());
                    // println!("{}", res.reason.unwrap());
                    // println!("{}", status.unwrap());
                    // println!("{}", read_size);
                },
                Err(e) => {
                    
                }
            }

            
            Ok(())
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

    use crate::{BoxedClone, PipelineStep};

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

        fn start(&self) {}
    }

    impl BoxedClone for WssDestination {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(WssDestination::new(&self.address))
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
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.as_bytes().len());
                        Ok(data.as_bytes().len())
                    },
                    Message::Binary(data) => unsafe {
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                        Ok(data.len())
                    },
                    Message::Close(e) => {
                        let errno = std::io::Error::last_os_error();
                        Err(errno)
                        // Err("{}", e.unwrap().code);
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => Ok(0),
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
