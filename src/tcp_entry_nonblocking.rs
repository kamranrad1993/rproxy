pub mod tcp_entry_nonblocking {
    use crate::{Entry, Pipeline};
    use http::Uri;
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::{self, Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
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
                        let client_key = self.connections.len() + self.listener_key;

                        self.connections
                            .insert(client_key, (client, client_address));

                        self.poller
                            .modify(&self.listener, Event::readable(self.listener_key))
                            .unwrap();

                        let mut cloned_self = self.clone();
                        thread::spawn(move || {
                            cloned_self.handle_connection(ev).unwrap();
                        });
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
        fn handle_connection(&mut self, event: Event) -> io::Result<()> {
            let client = self.connections.get_mut(&event.key).unwrap();

            unsafe {
                self.poller.add(&client.0, Event::all(event.key))?;
            }
            let mut events = Events::new();

            loop {
                self.poller.wait(&mut events, None).unwrap();

                for ev in events.iter() {
                    if ev.key == event.key {
                        if ev.readable {
                            match TcpEntryNonBlocking::len(&mut client.0) {
                                Ok(len) => {
                                    if len > 0 {
                                        let mut buf = vec![0; len];
                                        match client.0.read_exact(&mut buf) {
                                            Ok(_) => {
                                                let len = buf.len();
                                                let final_size = self.pipeline.write(buf).unwrap();
                                            }
                                            Err(e) => {
                                                println!("Error reading from stream: {}", e);
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
                                    match client.0.write_all(&data) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            println!("Error writing to stream: {}", e);
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
