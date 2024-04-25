pub mod websocket_entry {
    use inline_colorization::{color_reset, color_yellow};
    use regex::Regex;
    use std::{
        io::{Read, Result, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
        sync::{Arc, Mutex},
    };
    use tungstenite::{accept, http::Uri, protocol::Role, Message, WebSocket};

    use crate::{pipeline_module::pipeline, Entry, Pipeline};

    pub struct WebsocketEntry {
        tcp_server: TcpListener,
        tcp_streams: Arc<Mutex<Vec<(TcpStream, Pipeline)>>>,
        address: String,
        pipeline: Pipeline,
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
            // let r = server.accept().unwrap().0;
            // accept(r.try_clone().unwrap()).unwrap();

            WebsocketEntry {
                tcp_server: server,
                address: config,
                tcp_streams: Arc::new(Mutex::new(Vec::new())),
                pipeline: pipeline,
            }
        }

        fn read(&mut self){
            let mut locked_streams = self.tcp_streams.lock().unwrap();
            for i in (0..locked_streams.len()) {
                let len = self.len(&mut locked_streams[i].0).unwrap();
                let mut websocket =
                    WebSocket::from_raw_socket(locked_streams[i].0.try_clone().unwrap(), Role::Client, None);
                let mut buf : Vec<u8> = vec![0; len];

                let mut available: usize = 0;
                let result: i32 =
                    unsafe { libc::ioctl(locked_streams[i].0.as_raw_fd(), libc::FIONREAD, &mut available) };

                if result == -1 {
                    let errno = std::io::Error::last_os_error();
                    println!("{}", errno);
                } else if available == 0 {
                    
                } else {
                    let m = &mut websocket.read().unwrap();
                    match m {
                        Message::Text(data) => unsafe {
                            let length = std::cmp::min(data.as_bytes().len(), buf.len());
                            std::ptr::copy(
                                data.as_mut_ptr(),
                                buf.as_mut_ptr(),
                                data.as_bytes().len(),
                            );
                            locked_streams[i].0.write(buf.as_mut_slice());
                        },
                        Message::Binary(data) => unsafe {
                            let length = std::cmp::min(data.len(), buf.len());
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                            locked_streams[i].0.write(buf.as_mut_slice());
                        },
                        Message::Ping(_)
                        | Message::Pong(_)
                        | Message::Close(_)
                        | Message::Frame(_) => {},
                    }
                    // self.tcp_stream.read(buf)
                }
            }
        }

        fn write(&mut self) {
            let mut locked_streams = self.tcp_streams.lock().unwrap();
            for i in (0..locked_streams.len()) {
                let len = self.len(&mut locked_streams[i].0).unwrap();
                let mut websocket =
                WebSocket::from_raw_socket(locked_streams[i].0.try_clone().unwrap(), Role::Client, None);

                let data = locked_streams[i].1.read().unwrap();
                let msg = Message::Binary(data);
                let result = websocket.send(msg).unwrap();
            }
        }
        
        fn len(&self, stream: &mut TcpStream) -> std::io::Result<usize> {
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
    }

    impl WebsocketEntry {
        pub fn listen(&mut self) {
            for conn in self.tcp_server.incoming() {
                match conn {
                    Ok(conn) => {
                        let mut websocket = accept(conn.try_clone().unwrap()).unwrap();

                        self.tcp_streams
                            .lock()
                            .unwrap()
                            .push((conn, self.pipeline.clone()));
                    }
                    Err(e) => {
                        println!("{color_yellow}{}{color_reset}", e)
                    }
                }
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
