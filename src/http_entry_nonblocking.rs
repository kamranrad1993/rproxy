pub mod http_entry_nonblocking {
    use std::{
        collections::HashMap,
        io::{self, Read, Write},
        net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream},
        os::fd::AsRawFd,
        result,
        str::FromStr,
        sync::{
            mpsc::{Receiver, Sender},
            Arc, Mutex,
        },
        thread,
        time::{Duration, SystemTime},
    };

    use http::{request, Response, StatusCode, Uri};
    use hyper::client::{self, conn};
    use openssl::{base64, error, sha::sha256, string};
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::sync::mpsc::channel;
    use threadpool::ThreadPool;

    use crate::{
        pipeline_module::pipeline, read_request, write_response, Entry, IOError, Pipeline,
    };

    const CLIENT_TOKEN_HEADER: &str = "client_token";

    pub struct HttpEntryNonblocking {
        salt: String,
        expiration_time: Duration,
        poller: Poller,
        listener: TcpListener,
        listener_key: usize,
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Entry for HttpEntryNonblocking {
        fn new(config: String, pipeline: Pipeline, loop_time: u64) -> Self {
            let config: Vec<&str> = config.split('-').collect();
            let timeout = Duration::from_secs(u64::from_str(config[2]).unwrap());

            let re = Regex::new(r"((https|http)?:\/\/)([^:/$]{1,})(?::(\d{1,}))").unwrap();
            if !re.is_match(&config[0]) {
                panic!(
                    "unsupported config : {}. use with this format ws://host:port ",
                    config[0]
                )
            }

            let uri: Uri = config[0].parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let mut listener = TcpListener::bind(addr).unwrap();
            let poller = Poller::new().unwrap();

            unsafe {
                poller.add(&listener, Event::readable(1)).unwrap();
            }

            HttpEntryNonblocking {
                salt: String::from_str(config[1]).unwrap(),
                poller,
                listener,
                listener_key: 1,
                pipeline: pipeline,
                loop_time,
                expiration_time: timeout,
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
            // let (client_channel_tx, client_channel_rx) =
            //     channel::<(PollerKey, SocketAddr, Poller)>();
            let pipeline_mutex = Arc::new(Mutex::new(self.pipeline.clone()));

            let connections: HashMap<String, (SocketAddr, Pipeline, SystemTime)> = HashMap::new();
            let connectiond_mutex = Arc::new(Mutex::new(connections));

            loop {
                self.poller.wait(&mut events, None).unwrap();

                for ev in events.iter() {
                    if ev.key == self.listener_key {
                        let mut connection = self.listener.accept().unwrap();
                        connection.0.set_nonblocking(true).unwrap();

                        let pipeline_mutex = pipeline_mutex.clone();
                        let connectiond_mutex = connectiond_mutex.clone();
                        let salt = self.salt.clone();
                        thread::spawn(move || {
                            HttpEntryNonblocking::handle_connection(
                                connection.0,
                                connection.1,
                                pipeline_mutex,
                                salt,
                                connectiond_mutex,
                            )
                            .unwrap();
                        });

                        self.poller
                            .modify(&self.listener, Event::readable(self.listener_key))
                            .unwrap();
                    }
                }

                HttpEntryNonblocking::check_expiration(
                    connectiond_mutex.clone(),
                    self.expiration_time,
                );
            }
        }
    }

    impl Clone for HttpEntryNonblocking {
        fn clone(&self) -> Self {
            Self {
                salt: self.salt.clone(),
                poller: Poller::new().unwrap(),
                listener: self.listener.try_clone().unwrap(),
                listener_key: self.listener_key.clone(),
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time,
                expiration_time: self.expiration_time,
            }
        }
    }

    impl HttpEntryNonblocking {
        fn write_handshake(token: &str, connection: TcpStream) -> Result<(), IOError> {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(CLIENT_TOKEN_HEADER, token)
                .body(vec![0u8; 0])
                .unwrap();

            write_response(connection, response)?;
            println!("token: {} ", token);
            Ok(())
        }

        fn write_invalid_access(connection: TcpStream) -> Result<(), IOError> {
            let msg = "Invalid Token";
            let response = Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(msg.as_bytes().to_vec())
                .unwrap();

            write_response(connection, response)?;
            return Err(IOError::InvalidData(msg.to_string()));
        }

        fn write_existing_pipeline_error(connection: TcpStream) -> Result<(), IOError> {
            let msg = "Piepline Already Exists";
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(msg.as_bytes().to_vec())
                .unwrap();

            write_response(connection, response)?;
            return Err(IOError::InvalidData(msg.to_string()));
        }

        fn write_unsupported_http_method_error(connection: TcpStream) -> Result<(), IOError> {
            let msg = "Unsupported Http Method";
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(msg.as_bytes().to_vec())
                .unwrap();

            write_response(connection, response)?;
            return Err(IOError::InvalidData(msg.to_string()));
        }

        fn generate_token(ip: IpAddr, salt: &str) -> String {
            let mut hasher = openssl::sha::Sha256::new();
            let mut client_key = String::from_str(&ip.to_string()).unwrap();
            client_key.push_str(&salt);
            hasher.update(client_key.as_bytes());
            let client_key = hasher.finish();
            base64::encode_block(&client_key)
        }

        fn validate_token(ip: IpAddr, salt: &str, token: &str) -> bool {
            HttpEntryNonblocking::generate_token(ip, salt) == token
        }

        fn write_response(connection: TcpStream, data: Vec<u8>) -> Result<(), IOError> {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(http::header::CONTENT_LENGTH, data.len())
                .body(data)
                .unwrap();

            write_response(connection, response)?;
            Ok(())
        }

        fn write_content_len(connection: TcpStream, len: usize) -> Result<(), IOError> {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(http::header::CONTENT_LENGTH, len)
                .body(vec![0u8; 0])
                .unwrap();

            write_response(connection, response)?;
            Ok(())
        }

        fn handle_connection(
            mut connection: TcpStream,
            address: SocketAddr,
            pipeline_mutex: Arc<Mutex<Pipeline>>,
            salt: String,
            connections: Arc<Mutex<HashMap<String, (SocketAddr, Pipeline, SystemTime)>>>,
        ) -> Result<(), IOError> {
            let request = read_request(&mut connection)?;

            if !request.headers().contains_key(CLIENT_TOKEN_HEADER) {
                println!("new req {}", address.to_string());
                for (key, value) in request.headers() {
                    println!("{}:{}", key, value.to_str().unwrap());
                }
                println!("+++++++++++++++++++++++++++++++++");

                let token = HttpEntryNonblocking::generate_token(address.ip(), &salt);
                let mut connections = connections.as_ref().lock().unwrap();
                if connections.contains_key(token.clone().as_str()) {
                    // return HttpEntryNonblocking::write_existing_pipeline_error(connection);
                    connections.remove(token.clone().as_str());
                }
                let mut pipeline = pipeline_mutex.lock().as_mut().unwrap().clone();
                pipeline.start();
                connections.insert(token.clone(), (address, pipeline, SystemTime::now()));
                return HttpEntryNonblocking::write_handshake(&token.clone(), connection);
            } else {
                let token = request
                    .headers()
                    .get(CLIENT_TOKEN_HEADER)
                    .unwrap()
                    .as_bytes();
                let token = std::str::from_utf8(token).unwrap();

                if !HttpEntryNonblocking::validate_token(address.ip(), &salt, token) {
                    return HttpEntryNonblocking::write_invalid_access(connection);
                }
                let mut connections = connections.as_ref().lock().unwrap();

                match Some(request.method()) {
                    Some(&http::Method::GET) => {
                        if connections.contains_key(token) {
                            let mut pipeline = connections.get_mut(token).unwrap();
                            let data = request.body().to_vec();
                            if data.len() > 0 {
                                if let Err(e) = pipeline.1.write(data) {
                                    match e {
                                        IOError::InvalidConnection
                                        | IOError::InvalidBindAddress
                                        | IOError::UnknownError(_)
                                        | IOError::IoError(_)
                                        | IOError::ParseError
                                        | IOError::InvalidStep(_)
                                        | IOError::InvalidData(_)
                                        | IOError::Error(_) => return Err(e),
                                        IOError::EmptyData => {
                                            return HttpEntryNonblocking::write_response(
                                                connection,
                                                vec![0u8; 0],
                                            );
                                        }
                                    }
                                }
                            }
                            pipeline.2 = SystemTime::now();

                            match pipeline.1.read() {
                                Ok(data) => {
                                    return HttpEntryNonblocking::write_response(connection, data);
                                }
                                Err(e) => match e {
                                    IOError::InvalidConnection
                                    | IOError::InvalidBindAddress
                                    | IOError::UnknownError(_)
                                    | IOError::IoError(_)
                                    | IOError::ParseError
                                    | IOError::InvalidStep(_)
                                    | IOError::InvalidData(_)
                                    | IOError::Error(_) => return Err(e),
                                    IOError::EmptyData => {
                                        return HttpEntryNonblocking::write_response(
                                            connection,
                                            vec![0u8; 0],
                                        );
                                    }
                                },
                            }
                        } else {
                            return HttpEntryNonblocking::write_invalid_access(connection);
                        }
                    }
                    Some(&http::Method::HEAD) => {
                        if connections.contains_key(token) {
                            let mut pipeline = connections.get_mut(token).unwrap();
                            // pipeline.1.write(request.body().to_vec()).unwrap();
                            pipeline.2 = SystemTime::now();

                            let pipeline_len = pipeline.1.len()?;

                            return HttpEntryNonblocking::write_content_len(
                                connection,
                                pipeline_len,
                            );
                        } else {
                            return HttpEntryNonblocking::write_invalid_access(connection);
                        }
                    }
                    Some(_) | None => {
                        return HttpEntryNonblocking::write_unsupported_http_method_error(
                            connection,
                        );
                    }
                }
            }
        }

        fn check_expiration(
            connections: Arc<Mutex<HashMap<String, (SocketAddr, Pipeline, SystemTime)>>>,
            timeout: Duration,
        ) {
            let mut connections = connections.as_ref().lock().unwrap();
            let mut expired_token = vec![String::new(); 0];
            for (token, data) in connections.iter_mut() {
                let dur = SystemTime::now().duration_since(data.2).unwrap();
                if dur > timeout {
                    expired_token.push(token.clone());
                }
            }
            for token in expired_token {
                connections.remove(&token);
            }
        }
    }
}
