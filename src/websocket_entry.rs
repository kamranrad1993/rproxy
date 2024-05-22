pub mod websocket_entry {
    use http::Response;
    use regex::Regex;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
        thread,
        time::Duration,
    };
    use tungstenite::{
        accept,
        error::ProtocolError,
        handshake::{server, MidHandshake},
        http::Uri,
        stream, Error, Message, WebSocket,
    };

    use crate::{Entry, Pipeline};

    pub struct WebsocketEntry {
        tcp_server: TcpListener,
        address: String,
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Clone for WebsocketEntry {
        fn clone(&self) -> Self {
            Self {
                tcp_server: self.tcp_server.try_clone().unwrap(),
                address: self.address.clone(),
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time,
            }
        }
    }

    impl Entry for WebsocketEntry {
        fn new(config: String, pipeline: crate::Pipeline, loop_time: u64) -> Self {
            let re = Regex::new(r"((https|wss|ws|http)?:\/\/)([^:/$]{1,})(?::(\d{1,}))").unwrap();
            if !re.is_match(&config) {
                panic!(
                    "unsupported config : {}. use with this format ws://host:port ",
                    config
                )
            }

            let uri: Uri = config.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let server = TcpListener::bind(addr.clone()).unwrap();

            WebsocketEntry {
                tcp_server: server,
                address: config,
                pipeline: pipeline,
                loop_time: loop_time,
            }
        }

        fn len(stream: &mut dyn AsRawFd) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(stream.as_raw_fd(), libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn listen(&mut self) {
            for conn in self.tcp_server.incoming() {
                match conn {
                    Ok(conn) => {
                        println!("new client : {}", conn.peer_addr().unwrap());
                        let mut websocket = accept(conn.try_clone().unwrap());
                        match websocket {
                            Ok(websocket) => {
                                let mut cloned_self = self.clone();

                                let read_write_thread = thread::spawn(move || {
                                    cloned_self.handle_pipeline(websocket, conn);
                                });
                            }
                            Err(e) => {
                                println!("{}", e);
                                match e {
                                    tungstenite::HandshakeError::Interrupted(mid_handshake) => {
                                        println!("midhandshake");
                                    }
                                    tungstenite::HandshakeError::Failure(e) => {
                                        self.handle_handshake_error(e, conn);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", e)
                    }
                }
            }
        }
    }

    impl WebsocketEntry {
        fn handle_protocol_error(&self, error: ProtocolError, mut stream: TcpStream) {
            match error {
                tungstenite::error::ProtocolError::WrongHttpMethod => println!("WrongHttpMethod"),
                tungstenite::error::ProtocolError::WrongHttpVersion => println!("WrongHttpVersion"),
                tungstenite::error::ProtocolError::MissingConnectionUpgradeHeader => {
                    println!("MissingConnectionUpgradeHeader");
                    // let response = tungstenite::http::Response::new("Only WebSocket connections are welcome here");
                    let response = Response::builder()
                        .status(400)
                        //  .header("X-Custom-Foo", "Bar")
                        .body("Only WebSocket connections are welcome here")
                        .unwrap();
                    tungstenite::handshake::server::write_response(stream, &response).unwrap();
                }
                tungstenite::error::ProtocolError::MissingUpgradeWebSocketHeader => {
                    println!("MissingUpgradeWebSocketHeader")
                }
                tungstenite::error::ProtocolError::MissingSecWebSocketVersionHeader => {
                    println!("MissingSecWebSocketVersionHeader")
                }
                tungstenite::error::ProtocolError::MissingSecWebSocketKey => {
                    println!("MissingSecWebSocketKey")
                }
                tungstenite::error::ProtocolError::SecWebSocketAcceptKeyMismatch => {
                    println!("SecWebSocketAcceptKeyMismatch")
                }
                tungstenite::error::ProtocolError::JunkAfterRequest => println!("JunkAfterRequest"),
                tungstenite::error::ProtocolError::CustomResponseSuccessful => {
                    println!("CustomResponseSuccessful")
                }
                tungstenite::error::ProtocolError::InvalidHeader(_) => println!("InvalidHeader"),
                tungstenite::error::ProtocolError::HandshakeIncomplete => {
                    println!("HandshakeIncomplete")
                }
                tungstenite::error::ProtocolError::HttparseError(_) => println!("HttparseError"),
                tungstenite::error::ProtocolError::SendAfterClosing => println!("SendAfterClosing"),
                tungstenite::error::ProtocolError::ReceivedAfterClosing => {
                    println!("ReceivedAfterClosing")
                }
                tungstenite::error::ProtocolError::NonZeroReservedBits => {
                    println!("NonZeroReservedBits")
                }
                tungstenite::error::ProtocolError::UnmaskedFrameFromClient => {
                    println!("UnmaskedFrameFromClient")
                }
                tungstenite::error::ProtocolError::MaskedFrameFromServer => {
                    println!("MaskedFrameFromServer")
                }
                tungstenite::error::ProtocolError::FragmentedControlFrame => {
                    println!("FragmentedControlFrame")
                }
                tungstenite::error::ProtocolError::ControlFrameTooBig => {
                    println!("ControlFrameTooBig")
                }
                tungstenite::error::ProtocolError::UnknownControlFrameType(_) => {
                    println!("UnknownControlFrameType")
                }
                tungstenite::error::ProtocolError::UnknownDataFrameType(_) => {
                    println!("UnknownDataFrameType")
                }
                tungstenite::error::ProtocolError::UnexpectedContinueFrame => {
                    println!("UnexpectedContinueFrame")
                }
                tungstenite::error::ProtocolError::ExpectedFragment(_) => {
                    println!("ExpectedFragment")
                }
                tungstenite::error::ProtocolError::ResetWithoutClosingHandshake => {
                    println!("ResetWithoutClosingHandshake")
                }
                tungstenite::error::ProtocolError::InvalidOpcode(_) => println!("InvalidOpcode"),
                tungstenite::error::ProtocolError::InvalidCloseSequence => {
                    println!("InvalidCloseSequence")
                }
            }
        }

        fn handle_handshake_error(&self, error: Error, mut stream: TcpStream) {
            match error {
                tungstenite::Error::ConnectionClosed => {
                    println!("ConnectionClosed");
                }
                tungstenite::Error::AlreadyClosed => {
                    println!("AlreadyClosed");
                }
                tungstenite::Error::Io(_) => {
                    println!("Io");
                }
                tungstenite::Error::Tls(_) => {
                    println!("Tls");
                }
                tungstenite::Error::Capacity(_) => {
                    println!("Capacity");
                }
                tungstenite::Error::Protocol(protocol) => {
                    self.handle_protocol_error(protocol, stream);
                }
                tungstenite::Error::WriteBufferFull(_) => {
                    println!("WriteBufferFull");
                }
                tungstenite::Error::Utf8 => {
                    println!("Utf8");
                }
                tungstenite::Error::AttackAttempt => {
                    println!("AttackAttempt");
                }
                tungstenite::Error::Url(_) => {
                    println!("Url");
                }
                tungstenite::Error::Http(_) => {
                    println!("Http");
                }
                tungstenite::Error::HttpFormat(_) => {
                    println!("HttpFormat");
                }
            }
        }

        fn handle_pipeline_(&mut self, mut websocket: WebSocket<TcpStream>) {
            loop {
                // Read data from the WebSocket connection
                let mut msg = match websocket.read() {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Error reading WebSocket message: {}", e);
                        break;
                    }
                };

                match &mut msg {
                    Message::Text(data) => unsafe {
                        let mut vdata = vec![0; data.as_bytes().len()];
                        std::ptr::copy(
                            data.as_mut_ptr(),
                            vdata.as_mut_ptr(),
                            data.as_bytes().len(),
                        );
                        self.pipeline.write(vdata).unwrap();
                    },
                    Message::Binary(data) => unsafe {
                        let mut buf: Vec<u8> = vec![0; data.len()];
                        std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                        self.pipeline.write(buf).unwrap();
                    },
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                    }
                }

                if self.pipeline.read_available() {
                    let data = self.pipeline.read().unwrap();
                    if !data.is_empty() {
                        let msg = Message::Binary(data);
                        match websocket.send(msg) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error writing to stream: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        }

        fn handle_pipeline(&mut self, mut websocket: WebSocket<TcpStream>, mut stream: TcpStream) {
            loop {
                let len = WebsocketEntry::len(&mut stream).unwrap();
                if len > 0 {
                    match &mut websocket.read() {
                        Ok(m) => {
                            if m.len() > 0 {
                                match m {
                                    Message::Text(data) => unsafe {
                                        let mut vdata = vec![0; data.as_bytes().len()];
                                        std::ptr::copy(
                                            data.as_mut_ptr(),
                                            vdata.as_mut_ptr(),
                                            data.as_bytes().len(),
                                        );
                                        self.pipeline.write(vdata).unwrap();
                                    },
                                    Message::Binary(data) => unsafe {
                                        let mut buf: Vec<u8> = vec![0; data.len()];
                                        std::ptr::copy(
                                            data.as_mut_ptr(),
                                            buf.as_mut_ptr(),
                                            data.len(),
                                        );
                                        self.pipeline.write(buf).unwrap();
                                    },
                                    Message::Ping(_)
                                    | Message::Pong(_)
                                    | Message::Close(_)
                                    | Message::Frame(_) => {}
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error reading from stream: {}", e);
                            break;
                        }
                    }
                }

                if self.pipeline.read_available() {
                    let data = self.pipeline.read().unwrap();
                    if !data.is_empty() {
                        let msg = Message::Binary(data);
                        match websocket.send(msg) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error writing to stream: {}", e);
                                break;
                            }
                        }
                    }
                }

                std::thread::sleep(Duration::from_millis(self.loop_time));
            }
        }
    }
}
