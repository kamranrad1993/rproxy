pub mod websocket_entry {
    use inline_colorization::{color_reset, color_yellow};
    use regex::Regex;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
    };
    use tungstenite::{accept, http::Uri};

    use crate::{pipeline_module::pipeline, Entry, Pipeline};

    pub struct WebsocketEntry {
        tcp_server: TcpListener,
        tcp_streams: Vec<TcpStream>,
        address: String,
        pipeline: Pipeline
    }

    impl Read for WebsocketEntry {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            todo!()
        }
    }

    impl Write for WebsocketEntry {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            todo!()
        }

        fn flush(&mut self) -> std::io::Result<()> {
            todo!()
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
            // let r = server.accept().unwrap().0;
            // accept(r.try_clone().unwrap()).unwrap();

            WebsocketEntry {
                tcp_server: server,
                address: config,
                tcp_streams: Vec::new(),
                pipeline: pipeline
            }
        }

        fn len(&self) -> std::io::Result<usize> {
            todo!()
        }
    }

    impl WebsocketEntry {
        async fn listen(&mut self) {
            let future = async {
                for conn in self.tcp_server.incoming() {
                    match conn {
                        Ok(tcp_conn) => {
                            self.tcp_streams.push(tcp_conn);
                        }
                        Err(e) => {
                            println!("{color_yellow}{}{color_reset}", e)
                        }
                    }
                }
            };
            future.await;
        }
    }
}
