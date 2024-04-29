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
                let mut temp_buf = [0u8, 1];
                let result = stream.peek(&mut temp_buf);
                result.unwrap();

                let len = self.len(&mut stream).unwrap();
                if len > 0 {
                    let mut buf: Vec<u8> = vec![0; len];
                    stream.read(buf.as_mut_slice()).unwrap();
                    pipeline.write(buf).unwrap();
                }

                thread::sleep(Duration::from_millis(5));
                if self.pipeline.read_available() {
                    let mut data: Vec<u8> = pipeline.read().unwrap();
                    if data.len() > 0 {
                        stream.write(&data.as_mut_slice()).unwrap();
                    }
                }
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
}
