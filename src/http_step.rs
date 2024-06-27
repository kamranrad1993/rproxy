#[allow(non_snake_case, unused_variables, dead_code)]
pub mod http_step {
    use http::{method, Method, Response, StatusCode};
    use std::io::{self, Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::FromStr;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::{Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep};
    use crate::{read_response, write_request, BoxedClone};

    const CLIENT_TOKEN_HEADER: &str = "client_token";

    pub struct HttpStep {
        token: Option<String>,
        address: String,
        buffer: Vec<u8>,
    }

    impl PipelineStep for HttpStep {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 = unsafe {
                libc::ioctl(
                    self.get_stream().as_raw_fd(),
                    libc::FIONREAD,
                    &mut available,
                )
            };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else {
                Ok(available)
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            // println!("{}", direction);
        }

        fn start(&mut self) {}
    }

    impl BoxedClone for HttpStep {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(HttpStep::new(&self.address))
        }
    }

    impl Read for HttpStep {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut connection = self.make_connection()?;
            match self.token {
                Some(token) => {
                    let request = Request::builder()
                        .method(Method::GET)
                        .header(CLIENT_TOKEN_HEADER, self.token.unwrap())
                        .body(self.buffer)
                        .unwrap();

                    write_request(connection, &request)?;
                    let response = read_response(&mut connection)?;

                    match Some(response.status()) {
                        Some(StatusCode::OK) => {
                            self.buffer.clear();
                            
                        }
                        Some(StatusCode::FORBIDDEN) => {}
                        Some(StatusCode::BAD_REQUEST) => {}
                        None => {}
                    }

                    if response.status() != 200 {
                        match std::str::from_utf8(response.body()) {
                            Ok(msg) => {
                                return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
                            }
                            Err(e) => {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Invalid utf8",
                                ));
                            }
                        }
                    }

                    match response.headers().get(CLIENT_TOKEN_HEADER) {
                        Some(token) => {
                            self.token = Some(String::from_str(token.to_str().unwrap()).unwrap());
                            Ok(())
                        }
                        None => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Token Not Found",
                        )),
                    }
                }
                None => {
                    self.handshake(&mut connection)?;
                    self.read(buf)
                }
            }
        }
    }

    impl Write for HttpStep {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[allow(unreachable_code)]
    impl HttpStep {
        pub fn new(address: &str) -> Self {
            //handle errors
            HttpStep {
                token: None,
                address: String::from_str(address).unwrap(),
                buffer: vec![0u8, 0],
            }
        }

        fn make_connection(&self) -> io::Result<TcpStream> {
            let uri: Uri = self.address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            let port = uri.port().unwrap().as_u16();

            addr.push_str(":");
            addr.push_str(port.to_string().as_str());
            TcpStream::connect(addr)
        }

        fn handshake(&mut self, connection: &mut TcpStream) -> io::Result<()> {
            let request = Request::builder()
                .method(Method::GET)
                .body(vec![0u8; 0])
                .unwrap();

            write_request(connection, &request)?;
            let response = read_response(connection)?;
            if response.status() != 200 {
                match std::str::from_utf8(response.body()) {
                    Ok(msg) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8"));
                    }
                }
            }

            match response.headers().get(CLIENT_TOKEN_HEADER) {
                Some(token) => {
                    self.token = Some(String::from_str(token.to_str().unwrap()).unwrap());
                    Ok(())
                }
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Token Not Found",
                )),
            }
        }
    }
}
