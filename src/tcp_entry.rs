pub mod tcp_entry {
    use libc::c_int;
    use regex::Regex;
    use std::io;
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
        loop_time: u64,
    }

    impl Clone for TCPEntry {
        fn clone(&self) -> Self {
            Self {
                tcp_server: self.tcp_server.try_clone().unwrap(),
                address: self.address.clone(),
                pipeline: self.pipeline.clone(),
                loop_time: self.loop_time
            }
        }
    }

    impl Entry for TCPEntry {
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
            // server.set_nonblocking(true).expect("Cannot set non-blocking");

            TCPEntry {
                tcp_server: server,
                address: config,
                pipeline: pipeline,
                loop_time: loop_time
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
        fn is_tcp_connection_alive(&self, stream: &TcpStream) -> io::Result<bool> {
            let raw_fd = stream.as_raw_fd();

            // Call ioctl with SIOCOUTQ to get the amount of unsent data in the socket's output buffer
            let mut outq: c_int = 0;
            let res = unsafe { libc::ioctl(raw_fd, libc::SIOCOUTQNSD, &mut outq) };

            if res == -1 {
                // Error occurred while calling ioctl
                Err(io::Error::last_os_error())
            } else {
                // If there is unsent data in the output buffer, the connection is still alive
                Ok(outq > 0)
            }
        }

        fn handle_pipeline(&self, mut stream: TcpStream, mut pipeline: Pipeline) {
            loop {
                // match self.is_tcp_connection_alive(&stream) {
                //     Ok(result) => {

                //     },
                //     Err(e) => {
                //         println!("{e}");
                //         break;
                //     },
                // }

                match self.len(&mut stream) {
                    Ok(len) => {
                        if len > 0 {
                            let mut buf = vec![0; len];
                            match stream.read_exact(&mut buf) {
                                Ok(_) => {
                                    let len = buf.len();
                                    let final_size = pipeline.write(buf).unwrap();
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

                // std::thread::sleep(Duration::from_millis(10));

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

                std::thread::sleep(Duration::from_millis(self.loop_time));
            }
        }
    }
}
