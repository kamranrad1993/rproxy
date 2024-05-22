pub mod websocket_entry_nonblocking {
    use crate::{Entry, Pipeline};
    use http::{Request, Response};
    use openssl::sha::Sha1;
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::{self, Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::os::fd::AsRawFd;
    use std::str;
    use std::thread;
    use tungstenite::protocol::Role;
    use tungstenite::{
        accept,
        error::ProtocolError,
        handshake::{server, MidHandshake},
        http::Uri,
        stream, Error, Message, WebSocket,
    };

    pub struct WSEntryNonBlocking {
        poller: Poller,
        address: String,
        listener: TcpListener,
        listener_key: usize,
        connections: HashMap<usize, (TcpStream, SocketAddr)>,
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Entry for WSEntryNonBlocking {
        fn new(config: String, pipeline: Pipeline, loop_time: u64) -> Self {
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
            let mut listener = TcpListener::bind(addr).unwrap();
            listener.set_nonblocking(true).unwrap();
            let poller = Poller::new().unwrap();

            unsafe {
                poller.add(&listener, Event::readable(1)).unwrap();
            }

            WSEntryNonBlocking {
                poller,
                address: config,
                listener,
                listener_key: 1,
                connections: HashMap::new(),
                pipeline: pipeline,
                loop_time,
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
            let mut events = Events::new();

            loop {
                self.poller.wait(&mut events, None).unwrap();

                for ev in events.iter() {
                    if ev.key == self.listener_key {
                        let (client, client_address) = self.listener.accept().unwrap();

                        let mut websocket = accept(client.try_clone().unwrap());
                        match websocket {
                            Ok(websocket) => {
                                let client_key = self.connections.len() + self.listener_key;
                                self.connections
                                    .insert(client_key, (client, client_address));
                                let mut cloned_self = self.clone();

                                thread::spawn(move || {
                                    cloned_self.handle_connection(ev).unwrap();
                                });
                            }
                            Err(e) => {
                                println!("{}", e);
                                match e {
                                    tungstenite::HandshakeError::Interrupted(mid_handshake) => {
                                        println!("midhandshake");
                                    }
                                    tungstenite::HandshakeError::Failure(e) => {
                                        self.handle_handshake_error(e, client);
                                    }
                                }
                            }
                        }

                        self.poller
                            .modify(&self.listener, Event::readable(self.listener_key))
                            .unwrap();
                    }
                }
            }
        }
    }

    impl Clone for WSEntryNonBlocking {
        fn clone(&self) -> Self {
            let mut connections = HashMap::new();
            for data in self.connections.iter() {
                connections.insert(data.0.clone(), (data.1 .0.try_clone().unwrap(), data.1 .1));
            }
            Self {
                poller: Poller::new().unwrap(),
                address: self.address.clone(),
                listener: self.listener.try_clone().unwrap(),
                listener_key: self.listener_key.clone(),
                connections: connections,
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time,
            }
        }
    }

    impl WSEntryNonBlocking {
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

        fn handshake(mut stream: TcpStream) {
            let mut buffer = [0; 1024];
            let read_size = stream.read(&mut buffer).unwrap();

            // Parse the HTTP request
            let request = Request::builder().body(()).unwrap();
            let request_str = str::from_utf8(&buffer[..read_size]).unwrap();
            let headers: Vec<&str> = request_str.split("\r\n").collect();
            let mut websocket_key = String::new();

            for header in headers {
                if header.starts_with("Sec-WebSocket-Key:") {
                    websocket_key = header.split(": ").nth(1).unwrap().to_string();
                    break;
                }
            }

            if websocket_key.is_empty() {
                eprintln!("WebSocket key not found in headers");
                return;
            }

            // Create WebSocket accept key
            let magic_string = websocket_key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
            let mut hasher = Sha1::new();
            hasher.update(magic_string.as_bytes());
            let result = hasher.finish();
            let accept_key = base64::encode(result);

            // Send WebSocket handshake response
            let response = format!(
                "HTTP/1.1 101 Switching Protocols\r\n\
                         Connection: Upgrade\r\n\
                         Upgrade: websocket\r\n\
                         Sec-WebSocket-Accept: {}\r\n\r\n",
                accept_key
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }

        fn handle_connection(&mut self, event: Event) -> io::Result<()> {
            let client = self.connections.get_mut(&event.key).unwrap();
            // client.0.flush().unwrap();
            // client.0.set_nonblocking(true).unwrap();


            unsafe {
                self.poller.add(&client.0, Event::all(event.key))?;
            }
            let mut events = Events::new();

            // WSEntryNonBlocking::handshake(client.0.try_clone().unwrap());

            loop {
                self.poller.wait(&mut events, None).unwrap();

                for ev in events.iter() {
                    if ev.key == event.key {
                        let mut websocket = WebSocket::from_raw_socket(
                            client.0.try_clone().unwrap(),
                            Role::Client,
                            None,
                        );
                        if ev.readable {
                            match WSEntryNonBlocking::len(&mut client.0) {
                                Ok(len) => {
                                    if len > 0 {
                                        match &mut websocket.read() {
                                            Ok(m) => {
                                                if m.len() > 0 {
                                                    match m {
                                                        Message::Text(data) => unsafe {
                                                            let mut vdata =
                                                                vec![0; data.as_bytes().len()];
                                                            std::ptr::copy(
                                                                data.as_mut_ptr(),
                                                                vdata.as_mut_ptr(),
                                                                data.as_bytes().len(),
                                                            );
                                                            self.pipeline.write(vdata).unwrap();
                                                        },
                                                        Message::Binary(data) => unsafe {
                                                            let mut buf: Vec<u8> =
                                                                vec![0; data.len()];
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
                                }
                                Err(e) => {
                                    println!("Error reading from stream: {}", e);
                                }
                            }
                        }

                        if ev.writable {
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
                }

                self.poller
                    .modify(&client.0, Event::all(event.key))
                    .unwrap();
            }

            Ok(())
        }
    }
}
