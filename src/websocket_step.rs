#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_source {
    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};
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
    }

    impl Write for WebsocketSource {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            self.get_websocket().send(msg).unwrap();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
            // self.tcp_stream.flush()
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
                // self.tcp_stream.read(buf)
                let m = &mut self.get_websocket().read().unwrap();
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match m {
                    Message::Text(data) => {
                        unsafe {
                            let length = std::cmp::min(data.as_bytes().len(), buf.len());
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), length);
                            Ok(length)
                        }
                        // buf.copy_from_slice(data.as_bytes());
                    }
                    Message::Binary(data) => {
                        unsafe {
                            let length = std::cmp::min(data.len(), buf.len());
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), length);
                            Ok(length)
                        }
                        // buf.copy_from_slice(data.as_slice());
                        // buf = data.as_mut_slice();
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                        Ok(0)
                    }
                }
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
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            // println!("{}", direction);
        }
    }

    impl WebsocketSource {
        pub fn new(address: &str) -> Self {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let server = TcpListener::bind(addr).unwrap();
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

// pub mod rustls_wrapper {
//     use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
//     use rustls_pki_types::ServerName;

//     use std::{
//         convert::TryFrom,
//         io::{Read, Write},
//         sync::Arc,
//     };

//     use tungstenite::{
//         error::TlsError,
//         stream::{MaybeTlsStream, Mode},
//         Result,
//     };

//     pub fn wrap_stream<S>(
//         socket: S,
//         domain: &str,
//         mode: Mode,
//         tls_connector: Option<Arc<ClientConfig>>,
//     ) -> Result<MaybeTlsStream<S>>
//     where
//         S: Read + Write,
//     {
//         match mode {
//             Mode::Plain => Ok(MaybeTlsStream::Plain(socket)),
//             Mode::Tls => {
//                 let config = match tls_connector {
//                     Some(config) => config,
//                     None => {
//                         #[allow(unused_mut)]
//                         let mut root_store = RootCertStore::empty();

//                         #[cfg(feature = "rustls-tls-native-roots")]
//                         {
//                             let native_certs = rustls_native_certs::load_native_certs()?;
//                             let total_number = native_certs.len();
//                             let (number_added, number_ignored) =
//                                 root_store.add_parsable_certificates(native_certs);
//                             log::debug!("Added {number_added}/{total_number} native root certificates (ignored {number_ignored})");
//                         }
//                         #[cfg(feature = "rustls-tls-webpki-roots")]
//                         {
//                             root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
//                         }

//                         Arc::new(
//                             ClientConfig::builder()
//                                 .with_root_certificates(root_store)
//                                 .with_no_client_auth(),
//                         )
//                     }
//                 };
//                 let domain = ServerName::try_from(domain)
//                     .map_err(|_| TlsError::InvalidDnsName)?
//                     .to_owned();
//                 let client = ClientConnection::new(config, domain).map_err(TlsError::Rustls)?;
//                 let stream = StreamOwned::new(client, socket);

//                 Ok(MaybeTlsStream::Rustls(stream))
//             }
//         }
//     }
// }

#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use base64::write;
    use rustls::{ClientConnection, Connection, RootCertStore};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::sync::Arc;
    use std::time::Duration;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::client::{client, uri_mode};
    use tungstenite::handshake::MidHandshake;
    use tungstenite::http::{response, Request, Response, StatusCode, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::stream::MaybeTlsStream;
    use tungstenite::{client_tls, connect, ClientHandshake, Message, WebSocket};

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};

    // pub struct WebsocketDestination {
    //     tcp_stream: TcpStream,
    //     context: WebSocketContext,
    //     // tls_stream: Option<MaybeTlsStream<TcpStream>>,
    // }

    struct ClientConnectionWrapper {
        object: ClientConnection,
    }

    impl Read for ClientConnectionWrapper {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.object.reader().read(buf)
        }
    }

    impl Write for ClientConnectionWrapper {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.object.writer().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.object.writer().flush()
        }
    }

    pub enum Stream {
        tcp_stream(TcpStream),
        tls_connection(ClientConnectionWrapper),
    }

    pub struct WebsocketDestination {
        // tcp_stream: TcpStream,
        stream: Stream,
        fd: usize,
        context: WebSocketContext,
        // tls_stream: Option<MaybeTlsStream<TcpStream>>,
    }

    impl PipelineStep for WebsocketDestination {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Destination
        }

        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.fd as i32, libc::FIONREAD, &mut available) };
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
    }

    impl Read for WebsocketDestination {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.fd as i32, libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else if available == 0 {
                Ok(0)
            } else {
                // let m = &mut self.get_websocket().read().unwrap();
                let mut m: Option<Message> = None;
                match &mut self.stream {
                    Stream::tcp_stream(s) => {
                        m = Some(self.context.read(s).unwrap());
                    }
                    Stream::tls_connection(c) => {
                        m = Some(self.context.read(c).unwrap());
                    }
                }
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match &mut m.unwrap() {
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
            // let result = self.get_websocket().send(msg);

            let m: Option<Message> = None;
            match &mut self.stream {
                Stream::tcp_stream(s) => {
                    let result = self.context.write(s, msg);
                    match result {
                        Ok(_) => Ok(buf.len()),
                        Err(error) => {
                            panic!("{}", error);
                        }
                    }
                }
                Stream::tls_connection(c) => {
                    let result = self.context.write(c, msg);
                    match result {
                        Ok(_) => Ok(buf.len()),
                        Err(error) => {
                            panic!("{}", error);
                        }
                    }
                }
            }

            // match result {
            //     Ok(_) => Ok(buf.len()),
            //     Err(error) => {
            //         panic!("{}", error);
            //     }
            // }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            // self.get_websocket().flush().unwrap();
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
            let req = Request::builder().uri(address).body(()).unwrap();
            let l = client(req, connection.try_clone().unwrap()).unwrap();

            //handle errors
            WebsocketDestination {
                // tcp_stream: connection,
                stream: Stream::tcp_stream(connection.try_clone().unwrap()),
                fd: connection.as_raw_fd() as usize,
                context: WebSocketContext::new(Role::Client, None),
                // tls_stream: None,
            }
        }

        // pub fn get_websocket(&self) -> WebSocket<TcpStream> {
        //     WebSocket::from
        //     WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Client, None)
        // }

        pub fn new_tls(address: &str) -> Self {
            let mut tcp_stream: Option<TcpStream> = None;
            let mut tls_stream: Option<TcpStream> = None;
            let mut tls_connection: Option<ClientConnection> = None;
            let mut address = String::from(address);
            loop {
                let uri: Uri = address.parse::<Uri>().unwrap();
                let mut addr = String::from(uri.host().unwrap());
                let mut port = 0;
                if uri.port() != None {
                    port = uri.port().unwrap().as_u16();
                } else {
                    port = match uri.scheme_str() {
                        Some("ws") | Some("http") => 80,
                        Some("wss") | Some("https") => 443,
                        None | _ => {
                            panic!("unknow uri scheme")
                        }
                    };
                }
                addr.push_str(":");
                addr.push_str(port.to_string().as_str());
                tcp_stream = Some(TcpStream::connect(addr).unwrap());
                let req: tungstenite::http::Request<()> =
                    uri.clone().into_client_request().unwrap();

                let l = tcp_stream.as_ref().as_deref().unwrap().try_clone().unwrap();
                let result = convert_tls(l, String::from(uri.host().unwrap()));
                tls_stream= Some(result.0);
                tls_connection = Some(result.1);
                // tls_connection = Some(
                //     super::rustls_wrapper::wrap_stream(
                //         l,
                //         uri.host().unwrap(),
                //         tungstenite::stream::Mode::Tls,
                //         None,
                //     )
                //     .unwrap(),
                // );

                let handshake = ClientHandshake::start(tls_stream.as_ref().unwrap(), req, None)
                    .unwrap()
                    .handshake();
                match handshake {
                    Ok(websocket) => break,
                    Err(e) => match e {
                        tungstenite::HandshakeError::Interrupted(mid_handshake) => {
                            std::thread::sleep(Duration::from_millis(20));
                            mid_handshake.handshake().unwrap();
                            break;
                        }
                        tungstenite::HandshakeError::Failure(e) => {
                            match e {
                                tungstenite::Error::ConnectionClosed => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::AlreadyClosed => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Io(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Tls(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Capacity(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Protocol(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::WriteBufferFull(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Utf8 => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::AttackAttempt => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Url(e) => {
                                    println!("{}", e)
                                }
                                tungstenite::Error::Http(e) => {
                                    let b = e.body().as_ref().unwrap();
                                    println!("{}", std::str::from_utf8(b.as_slice()).unwrap());
                                    let new_location =
                                        e.headers().get("Location").unwrap().to_str().unwrap();
                                    address = String::from(new_location);
                                    println!("{new_location}");
                                    continue;
                                    // e.headers()
                                }
                                tungstenite::Error::HttpFormat(e) => {
                                    println!("{}", e)
                                }
                            }
                            break;
                        }
                    },
                }
            }


            WebsocketDestination {
                stream: Stream::tls_connection(ClientConnectionWrapper {
                    object: tls_connection.unwrap(),
                }),
                fd: tls_stream.as_ref().unwrap().as_raw_fd() as usize,
                context: WebSocketContext::new(Role::Client, None),
                // tls_stream: tls_connection,
            }
        }
    }

    fn convert_tls(mut stream: TcpStream, host: String) -> (TcpStream, ClientConnection) {
        let root_store = Arc::new(RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        ));
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store.as_ref().clone())
            .with_no_client_auth();
        config.key_log = Arc::new(rustls::KeyLogFile::new());
        config.dangerous().set_certificate_verifier(
            rustls::client::WebPkiServerVerifier::builder(root_store)
                .allow_unknown_revocation_status()
                .build()
                .unwrap(),
        );
        let server_name = host.try_into().unwrap();
        let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();
        let tls = rustls::Stream::new(&mut conn, &mut stream);
        let s = tls.sock.try_clone().unwrap();
        let c = *tls.conn;
        // ClientConnection::new(tls.conn., name)
        // tls.conn.complete_io(io)
        return (s, c);
    }
}
