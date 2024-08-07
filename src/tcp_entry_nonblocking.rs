pub mod tcp_entry_nonblocking {
    use crate::{Entry, IOError, Pipeline};
    use http::Uri;
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::os::fd::AsRawFd;
    use std::thread;
    use std::time::Duration;

    pub struct TcpEntryNonBlocking {
        poller: Poller,
        listener: TcpListener,
        listener_key: usize,
        connections: HashMap<usize, (TcpStream, SocketAddr)>,
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Entry for TcpEntryNonBlocking {
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
            let poller = Poller::new().unwrap();

            unsafe {
                poller.add(&listener, Event::readable(1)).unwrap();
            }

            TcpEntryNonBlocking {
                poller,
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
                            if let Err(e) = cloned_self.handle_connection(client_key) {
                                match e {
                                    IOError::InvalidConnection
                                    | IOError::InvalidBindAddress
                                    | IOError::UnknownError(_)
                                    | IOError::IoError(_)
                                    | IOError::ParseError
                                    | IOError::InvalidStep(_)
                                    | IOError::InvalidData(_)
                                    | IOError::Error(_) => {
                                        println!("{}", e);
                                        return;
                                    }
                                    IOError::EmptyData => {}
                                }
                            }
                        });

                        self.poller
                            .modify(&self.listener, Event::readable(self.listener_key))
                            .unwrap();
                    }
                }
            }
        }
    }

    impl Clone for TcpEntryNonBlocking {
        fn clone(&self) -> Self {
            let mut connections = HashMap::new();
            for data in self.connections.iter() {
                connections.insert(data.0.clone(), (data.1 .0.try_clone().unwrap(), data.1 .1));
            }
            Self {
                poller: Poller::new().unwrap(),
                listener: self.listener.try_clone().unwrap(),
                listener_key: self.listener_key.clone(),
                connections: connections,
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time,
            }
        }
    }

    impl TcpEntryNonBlocking {
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
            let mut is_connected = true;

            loop {
                thread::sleep(Duration::from_millis(10));
                self.poller.wait(&mut events, None)?;

                for ev in events.iter() {
                    if ev.key == client_key {
                        if ev.readable {
                            match TcpEntryNonBlocking::len(&mut client.0) {
                                Ok(len) => {
                                    if len > 0 {
                                        let mut buf = vec![0; len];
                                        match client.0.read_exact(&mut buf) {
                                            Ok(_) => {
                                                let len = buf.len();
                                                match self.pipeline.write(buf) {
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
                                                }
                                            }
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

                        if ev.writable {
                            if self.pipeline.read_available() {
                                match self.pipeline.read() {
                                    Ok(data) => {
                                        if !data.is_empty() {
                                            if let Err(e) = client.0.write(&data) {
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
            client.0.shutdown(Shutdown::Both)?;
            println!(
                "client disconnected, key : {}, address : {} ",
                client_key, client.1
            );
            Ok(())
        }
    }
}
