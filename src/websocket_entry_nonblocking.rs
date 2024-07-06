pub mod websocket_entry_nonblocking {
    use crate::http_tools::http_tools;
    use crate::{get_available_bytes, read_request, write_response, Entry, IOError, Pipeline};
    use bytes::{self, BytesMut};
    use http::{response, Request, Response, Version};
    use openssl::sha::Sha1;
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::os::fd::AsRawFd;
    use std::str;
    use std::thread;
    use std::time::Duration;
    use tokio_util::codec::{Decoder, Encoder};
    use tungstenite::handshake;
    use tungstenite::{error::ProtocolError, http::Uri, Error};
    use websocket_codec::{self, Message, MessageCodec};

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
            let listener = TcpListener::bind(addr).unwrap();
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
                        let client_key = self.connections.len() + self.listener_key + 1;

                        self.connections
                            .insert(client_key, (client, client_address));
                        let mut cloned_self = self.clone();

                        thread::spawn(move || {
                            cloned_self.handle_connection(client_key);
                        });

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
        fn handshake(mut stream: TcpStream) -> std::io::Result<()> {
            let read_size = get_available_bytes(&mut stream)?;
            let mut buffer = vec![0u8; read_size];

            let request = read_request(&mut stream)?;
            let mut websocket_key = String::new();

            for (header_name, header_value) in request.headers() {
                if (header_name.as_str() == "Sec-WebSocket-Key")
                    | (header_name.as_str() == "sec-websocket-key")
                {
                    websocket_key = std::str::from_utf8(header_value.as_bytes())
                        .unwrap()
                        .to_string();
                    break;
                }
            }

            if websocket_key.is_empty() {
                let e = io::Error::new(
                    io::ErrorKind::NotFound,
                    "WebSocket key not found in headers",
                );

                let msg = "only websocket connection accpted on this server."
                    .as_bytes()
                    .to_vec();
                let response = response::Builder::new()
                    .version(Version::HTTP_11)
                    .status(200)
                    .header("Connection", "Accepted")
                    .header("custom-header", "1")
                    .body(msg)
                    .unwrap();

                write_response(stream, response)?;
                // std::thread::sleep(Duration::from_millis(50));

                return Err(e);
            }

            let magic_string = websocket_key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
            let mut hasher = Sha1::new();
            hasher.update(magic_string.as_bytes());
            let result = hasher.finish();
            let accept_key = base64::encode(result);

            let response = response::Builder::new()
                .version(Version::HTTP_11)
                .status(101)
                .header("Connection", "Upgrade")
                .header("Accept", "text/html; charset=utf-8")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Accept", accept_key)
                .header("Connection", "keep-alive")
                .header("Keep-Alive", "timeout=6553600")
                .header("Upgrade-Insecure-Requests", "1")
                .header("custom-header", "1")
                .body(vec![0u8; 0])
                .unwrap();

            write_response(stream, response)?;
            Ok(())
        }

        fn handle_connection(&mut self, client_key: usize) -> Result<(), IOError> {
            self.pipeline.start();

            let client = self.connections.get_mut(&client_key).unwrap();
            client.0.set_nonblocking(true)?;

            println!(
                "new client connected, key : {}, address : {} ",
                client_key, client.1
            );

            unsafe {
                self.poller.add(&client.0, Event::all(client_key))?;
            }
            let mut events = Events::new();

            let mut handshaked = false;
            let mut is_connected = true;

            loop {
                thread::sleep(Duration::from_millis(10));
                self.poller.wait(&mut events, None)?;

                for ev in events.iter() {
                    if ev.key == client_key {
                        if ev.readable {
                            if !handshaked {
                                if let Err(e) = WSEntryNonBlocking::handshake(client.0.try_clone()?)
                                {
                                    is_connected = false;
                                    break;
                                } else {
                                    handshaked = true;
                                    continue;
                                }
                            }
                            match WSEntryNonBlocking::len(&mut client.0) {
                                Ok(len) => {
                                    if len > 0 {
                                        let mut buf: BytesMut = BytesMut::new();
                                        buf.resize(len, 0u8);
                                        if let Err(e) = client.0.read(buf.as_mut()) {
                                            println!("Error reading from stream: {}", e);
                                            is_connected = false;
                                            break;
                                        }

                                        let mut msgc = websocket_codec::MessageCodec::server()
                                            .decode(&mut buf);

                                        match &mut msgc {
                                            Ok(msgc) => match msgc {
                                                Some(msg) => match msg.opcode() {
                                                    websocket_codec::Opcode::Text
                                                    | websocket_codec::Opcode::Binary => match self
                                                        .pipeline
                                                        .write(msg.data().to_vec())
                                                    {
                                                        Ok(size) => {}
                                                        Err(e) => match e {
                                                            IOError::InvalidConnection
                                                            | IOError::InvalidBindAddress
                                                            | IOError::UnknownError(_)
                                                            | IOError::IoError(_)
                                                            | IOError::ParseError
                                                            | IOError::InvalidStep(_)
                                                            | IOError::InvalidData(_)
                                                            | IOError::Error(_) => {
                                                                return Err(e);
                                                            }
                                                            IOError::EmptyData => {}
                                                        },
                                                    },
                                                    websocket_codec::Opcode::Close => {
                                                        is_connected = false;
                                                        break;
                                                    }
                                                    websocket_codec::Opcode::Ping
                                                    | websocket_codec::Opcode::Pong => {}
                                                },
                                                None => {
                                                    println!(
                                                        "Error reading from stream: {}",
                                                        "Invalid Websocket Message"
                                                    );
                                                    is_connected = false;
                                                    break;
                                                }
                                            },
                                            Err(e) => {
                                                println!("Error reading from stream: {}", e);
                                                is_connected = false;
                                                break;
                                            }
                                        }
                                    } else {
                                        println!("Error reading from stream: {}", "Zero Length");
                                        is_connected = false;
                                        break;
                                    }
                                }
                                Err(e) => {
                                    println!("Error reading from stream: {}", e);
                                    is_connected = false;
                                    break;
                                }
                            }
                        }

                        if ev.writable && handshaked {
                            if self.pipeline.read_available() {
                                match self.pipeline.read() {
                                    Ok(data) => {
                                        if !data.is_empty() {
                                            let msg = Message::binary(data);
                                            let mut buf: BytesMut = BytesMut::new();
                                            MessageCodec::server().encode(&msg, &mut buf)?;

                                            if let Err(e) = client.0.write(buf.to_vec().as_slice())
                                            {
                                                println!("Error writing to stream: {}", e);
                                                is_connected = false;
                                                break;
                                            }
                                            if let Err(e) = client.0.flush() {
                                                println!("Error flush stream: {}", e);
                                                is_connected = false;
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => match e {
                                        IOError::InvalidConnection
                                        | IOError::InvalidBindAddress
                                        | IOError::UnknownError(_)
                                        | IOError::IoError(_)
                                        | IOError::ParseError
                                        | IOError::InvalidStep(_)
                                        | IOError::InvalidData(_)
                                        | IOError::Error(_) => {
                                            client.0.shutdown(Shutdown::Both)?;
                                            return Err(e);
                                        }
                                        IOError::EmptyData => {}
                                    },
                                }
                            }
                        } else {
                            // is_connected = false;
                            // break;
                        }
                    }
                }

                if !is_connected {
                    break;
                }

                self.poller.modify(&client.0, Event::all(client_key))?;
            }

            println!(
                "client disconnected, key : {}, address : {} ",
                client_key, client.1
            );
            client.0.shutdown(Shutdown::Both)?;
            Ok(())
        }
    }
}
