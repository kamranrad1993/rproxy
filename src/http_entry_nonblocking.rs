pub mod http_entry_nonblocking {
    use std::{
        collections::HashMap,
        io::{self, Read, Write},
        net::{Shutdown, SocketAddr, TcpListener, TcpStream},
        os::fd::AsRawFd,
        result,
        sync::{
            mpsc::{Receiver, Sender},
            Arc, Mutex,
        },
        thread,
        time::Duration,
    };

    use http::{request, Uri};
    use polling::{Event, Events, Poller};
    use regex::Regex;
    use std::sync::mpsc::channel;
    use threadpool::ThreadPool;

    use crate::{pipeline_module::pipeline, read_request, Entry, Pipeline};

    type ConnectionKey = usize;

    pub struct HttpEntryNonblocking {
        poller: Poller,
        listener: TcpListener,
        listener_key: ConnectionKey,
        pipeline: Pipeline,
        loop_time: u64,
    }

    impl Entry for HttpEntryNonblocking {
        fn new(config: String, pipeline: Pipeline, loop_time: u64) -> Self {
            let re = Regex::new(r"((https|http)?:\/\/)([^:/$]{1,})(?::(\d{1,}))").unwrap();
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

            HttpEntryNonblocking {
                poller,
                listener,
                listener_key: 1,
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
            let (client_channel_tx, client_channel_rx) =
                channel::<(ConnectionKey, SocketAddr, Poller)>();
            let pipeline_mutex = Arc::new(Mutex::new(self.pipeline.clone()));

            let connections: HashMap<ConnectionKey, (ConnectionKey, SocketAddr, Poller, Pipeline)> =
                HashMap::new();
            let connectiond_mutex = Arc::new(Mutex::new(connections));

            loop {
                self.poller.wait(&mut events, None).unwrap();

                for ev in events.iter() {
                    if ev.key == self.listener_key {
                        let mut connection = self.listener.accept().unwrap();

                        let pipeline_mutex = pipeline_mutex.clone();
                        let connectiond_mutex = connectiond_mutex.clone();
                        thread::spawn(move || {
                            HttpEntryNonblocking::handle_connection(
                                connection.0,
                                connection.1,
                                pipeline_mutex,
                                connectiond_mutex,
                            );
                        });

                        self.poller
                            .modify(&self.listener, Event::readable(self.listener_key))
                            .unwrap();
                    }
                }
            }
        }
    }

    impl Clone for HttpEntryNonblocking {
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

    impl HttpEntryNonblocking {
        fn handle_connection(
            mut connection: TcpStream,
            address: SocketAddr,
            pipeline_mutex: Arc<Mutex<Pipeline>>,
            connections: Arc<
                Mutex<HashMap<ConnectionKey, (ConnectionKey, SocketAddr, Poller, Pipeline)>>,
            >,
        ) -> io::Result<()> {
            let request = read_request(&mut connection)?;

            if request.headers().contains_key("client_key") {

            }else{
                
            }

            // self.connections
            // .insert(client_key, (client_key, client, client_address, self.pipeline.clone(), self.poller));

            // self.pipeline.start();

            // let client = self.connections.get_mut(&client_key).unwrap();
            // client.0.set_nonblocking(true).unwrap();

            // println!(
            //     "new client connected, key : {}, address : {} ",
            //     client_key, client.1
            // );

            // unsafe {
            //     self.poller.add(&client.0, Event::all(client_key))?;
            // }
            // let mut events = Events::new();
            // let mut is_connected = true;

            // loop {
            //     thread::sleep(Duration::from_millis(10));
            //     self.poller.wait(&mut events, None).unwrap();

            //     for ev in events.iter() {
            //         if ev.key == client_key {
            //             if ev.readable {
            //                 match HttpEntryNonblocking::len(&mut client.0) {
            //                     Ok(len) => {
            //                         if len > 0 {
            //                             let mut buf = vec![0; len];
            //                             match client.0.read_exact(&mut buf) {
            //                                 Ok(_) => {
            //                                     let len = buf.len();
            //                                     let final_size = self.pipeline.write(buf).unwrap();
            //                                 }
            //                                 Err(e) => {
            //                                     println!("Error reading from stream: {}", e);
            //                                     is_connected = false;
            //                                     break;
            //                                 }
            //                             }
            //                         } else {
            //                             println!("Error reading from stream: {}", "Zero Length");
            //                             is_connected = false;
            //                             break;
            //                         }
            //                     }
            //                     Err(e) => {
            //                         println!("Error reading from stream: {}", e);
            //                         is_connected = false;
            //                         break;
            //                     }
            //                 }
            //             }

            //             if ev.writable {
            //                 if self.pipeline.read_available() {
            //                     let data = self.pipeline.read().unwrap();
            //                     if !data.is_empty() {
            //                         if let Err(e) = client.0.write(&data) {
            //                             println!("Error writing to stream: {}", e);
            //                             is_connected = false;
            //                             break;
            //                         }

            //                         if let Err(e) = client.0.flush() {
            //                             println!("Error flush stream: {}", e);
            //                             is_connected = false;
            //                             break;
            //                         }
            //                     }
            //                 }
            //             } else {
            //                 // is_connected = false;
            //                 // break;
            //             }
            //         }
            //     }

            //     if !is_connected {
            //         break;
            //     }

            //     self.poller
            //         .modify(&client.0, Event::all(client_key))
            //         .unwrap();
            // }
            // client.0.shutdown(Shutdown::Both).unwrap();
            // println!(
            //     "client disconnected, key : {}, address : {} ",
            //     client_key, client.1
            // );
            Ok(())
        }
    }
}
