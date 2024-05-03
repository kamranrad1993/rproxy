pub mod tcp_entry {
    use regex::Regex;
    use std::str::{self, FromStr};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
        thread,
        time::Duration,
    };
    use tungstenite::http::Uri;

    use crate::{Entry, Pipeline};

    pub struct TCPEntry {
        tcp_server: TcpListener,
        address: String,
        pipeline: Pipeline,
    }

    impl Clone for TCPEntry {
        fn clone(&self) -> Self {
            Self {
                tcp_server: self.tcp_server.try_clone().unwrap(),
                address: self.address.clone(),
                pipeline: self.pipeline.clone(),
            }
        }
    }

    impl Entry for TCPEntry {
        fn new(config: String, pipeline: crate::Pipeline) -> Self {
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
            // server.set_nonblocking(true).expect("Cannot set non-blocking");

            TCPEntry {
                tcp_server: server,
                address: config,
                pipeline: pipeline,
            }
        }

        fn len(&self, stream: &mut dyn AsRawFd) -> std::io::Result<usize> {
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

                        let cloned_pipeline = self.pipeline.clone();
                        let cloned_self = self.clone();

                        let read_write_thread = thread::spawn(move || {
                            cloned_self.handle_pipeline(conn, cloned_pipeline);
                        });
                    }
                    Err(e) => {
                        println!("{}", e)
                    }
                }
            }
        }
    }

    impl TCPEntry {

        fn handle_pipeline(&self, mut stream: TcpStream, mut pipeline: Pipeline) {
            loop {
                match self.len(&mut stream) {
                    Ok(len) => {
                        if len > 0 {
                            let mut buf = vec![0; len];
                            match stream.read_exact(&mut buf) {
                                Ok(_) => {
                                    let final_size = pipeline.write(buf).unwrap();
                                }
                                Err(e) => {
                                    println!("Error reading from stream: {}", e);
                                    break;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        println!("Error reading from stream: {}", e);
                    },
                }
        
                if self.pipeline.read_available() {
                    let data = pipeline.read().unwrap(); 
                    if !data.is_empty() {
                        match stream.write_all(&data) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error writing to stream: {}", e);
                                break;
                            }
                        }
                    }
                    
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        }   
    }
}
