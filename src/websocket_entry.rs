pub mod websocket_entry {
    use regex::Regex;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
        thread,
        time::Duration,
    };
    use tungstenite::{accept, http::Uri, Message, WebSocket};

    use crate::{Entry, Pipeline};

    pub struct WebsocketEntry {
        tcp_server: TcpListener,
        address: String,
        pipeline: Pipeline,
    }

    impl Clone for WebsocketEntry {
        fn clone(&self) -> Self {
            Self {
                tcp_server: self.tcp_server.try_clone().unwrap(),
                address: self.address.clone(),
                pipeline: self.pipeline.clone(),
            }
        }
    }

    impl Entry for WebsocketEntry {
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

            WebsocketEntry {
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
                        let mut websocket = accept(conn.try_clone().unwrap()).unwrap();

                        let cloned_pipeline = self.pipeline.clone();
                        let cloned_self = self.clone();

                        let read_write_thread = thread::spawn(move || {
                            cloned_self.handle_pipeline(websocket, conn, cloned_pipeline);
                        });
                    }
                    Err(e) => {
                        println!("{}", e)
                    }
                }
            }
        }
    }

    impl WebsocketEntry {
        fn handle_pipeline(
            &self,
            mut websocket: WebSocket<TcpStream>,
            mut stream: TcpStream,
            mut pipeline: Pipeline,
        ) {
            loop {
                let len = self.len(&mut stream).unwrap();
                if len > 0 {
                    let m = &mut websocket.read().unwrap();
                    if m.len() > 0 {
                        match m {
                            Message::Text(data) => unsafe {
                                let mut buf: Vec<u8> = vec![0; data.as_bytes().len()];
                                std::ptr::copy(
                                    data.as_mut_ptr(),
                                    buf.as_mut_ptr(),
                                    data.as_bytes().len(),
                                );
                                let mut vdata = Vec::<u8>::from_raw_parts(
                                    data.as_mut_ptr(),
                                    data.as_bytes().len(),
                                    data.as_bytes().len(),
                                );
                                pipeline.write(vdata).unwrap();
                            },
                            Message::Binary(data) => unsafe {
                                let mut buf: Vec<u8> = vec![0; data.len()];
                                std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                                pipeline.write(buf).unwrap();
                            },
                            Message::Ping(_)
                            | Message::Pong(_)
                            | Message::Close(_)
                            | Message::Frame(_) => {}
                        }
                    }
                }

                let data = pipeline.read().unwrap();
                if data.len() > 0 {
                    let msg = Message::Binary(data);
                    websocket.send(msg).unwrap();
                }
                thread::sleep(Duration::from_millis(5));
            }
        }

        // async fn listen(&mut self) {
        //     let future = async {
        //         for conn in self.tcp_server.incoming() {
        //             match conn {
        //                 Ok(tcp_conn) => {
        //                     self.tcp_streams.push(tcp_conn);
        //                 }
        //                 Err(e) => {
        //                     println!("{color_yellow}{}{color_reset}", e)
        //                 }
        //             }
        //         }
        //     };
        //     future.await;
        // }
    }
}
