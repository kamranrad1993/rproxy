#[allow(non_snake_case, unused_variables, dead_code)]
pub mod http_step {
    use http::{method, HeaderValue, Method, Response, StatusCode, Version};
    use std::io::{self, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::FromStr;
    use tungstenite::http::{Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::pipeline_module::pipeline::{IOError, PipelineDirection, PipelineStep, Read};
    use crate::{read_response, write_request, BoxedClone};

    const CLIENT_TOKEN_HEADER: &str = "client_token";

    pub struct HttpStep {
        token: Option<String>,
        address: String,
        buffer: Vec<u8>,
    }

    impl PipelineStep for HttpStep {
        fn len(&mut self) -> std::io::Result<usize> {
            // Ok(self.buffer.len())
            if self.buffer.len() != 0 {
                Ok(self.buffer.len())
            } else {
                let mut request = Request::builder()
                    .method(http::Method::HEAD)
                    .version(Version::HTTP_11)
                    .body(vec![0u8; 0])
                    .unwrap();
                let response = self.write_request(&mut request)?;
                if response
                    .headers()
                    .contains_key(http::header::CONTENT_LENGTH)
                {
                    let l = response
                        .headers()
                        .get(http::header::CONTENT_LENGTH)
                        .unwrap()
                        .to_str();

                    let len = usize::from_str(
                        response
                            .headers()
                            .get(http::header::CONTENT_LENGTH)
                            .unwrap()
                            .to_str()
                            .unwrap(),
                    )
                    .unwrap();
                    Ok(len)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "Content-Length Header Not Found",
                    ))
                }
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
        fn read(&mut self) -> Result<Vec<u8>, IOError> {
            let mut connection = self.make_connection()?;

            let mut request = Request::builder()
                .method(Method::GET)
                .version(Version::HTTP_11)
                .header(http::header::CONTENT_LENGTH, self.buffer.len())
                .body(self.buffer.to_vec())
                .unwrap();
            let response = self.write_request(&mut request)?;

            match Some(response.status()) {
                Some(StatusCode::OK) => {
                    let wsize = response.body().len();
                    self.buffer.clear();
                    Ok(response.body().clone())
                }
                Some(_) | None => match std::str::from_utf8(response.body()) {
                    Ok(msg) => {
                        return Err(IOError::UnknownError(msg.to_string()));
                    }
                    Err(e) => {
                        return Err(IOError::ParseError);
                    }
                },
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
                buffer: vec![0u8; 0],
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

        fn write_request(
            &mut self,
            request: &mut Request<Vec<u8>>,
        ) -> io::Result<Response<Vec<u8>>> {
            let mut connection = self.make_connection()?;
            match self.token.clone() {
                Some(token) => {
                    request
                        .headers_mut()
                        .append(CLIENT_TOKEN_HEADER, HeaderValue::from_str(&token).unwrap());

                    write_request(&mut connection, request)?;
                    let response = read_response(&mut connection)?;
                    match Some(response.status()) {
                        Some(StatusCode::FORBIDDEN) => {
                            match std::str::from_utf8(response.body()) {
                                Ok(msg) => {
                                    println!("{}", msg);
                                    self.handshake(connection)?;
                                    self.write_request(request);
                                    // return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
                                }
                                Err(e) => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "Invalid utf8",
                                    ));
                                }
                            }
                        }
                        Some(StatusCode::OK) => {
                            return Ok(response);
                        }
                        Some(_) | None => match std::str::from_utf8(response.body()) {
                            Ok(msg) => {
                                return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
                            }
                            Err(e) => {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Invalid utf8",
                                ));
                            }
                        },
                    }
                    Ok(response)
                }
                None => {
                    self.handshake(connection)?;
                    self.write_request(request)
                }
            }
        }

        fn handshake(&mut self, mut connection: TcpStream) -> io::Result<()> {
            let request = Request::builder()
                .method(Method::GET)
                .version(Version::HTTP_11)
                .body(vec![0u8; 0])
                .unwrap();

            write_request(&mut connection, &request)?;
            let response = read_response(&mut connection)?;
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
