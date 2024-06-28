pub mod http_tools {
    use http::{request, response, version, Request, Response, Version};
    use rand::seq;
    use tungstenite::buffer;
    use std::{
        io::{self, Read, Result, Write},
        iter::{self, Iterator},
        os::fd::AsRawFd,
        str::{self, Utf8Error},
    };

    // #[test]
    // pub fn test_version() -> Result<()> {
    //     let v = http::HeaderValue::from_str("HTTP/1.1").unwrap();
    //     assert_eq!(Version::HTTP_11, "HTTP/1.1");
    //     Ok(())
    // }

    pub fn write_response<T: Write, B: AsRef<[u8]>>(
        mut stream: T,
        response: Response<B>,
    ) -> Result<usize> {
        let mut buffer = Vec::new();

        write!(buffer, "HTTP/1.1 {}\r\n", response.status(),)?;

        for (key, value) in response.headers() {
            write!(buffer, "{}: {}\r\n", key, value.to_str().unwrap())?;
        }

        write!(buffer, "\r\n")?;

        buffer.write(response.body().as_ref())?;

        stream.write(buffer.as_ref())
    }

    pub fn write_request<T: Write>(mut stream: T, request: &Request<Vec<u8>>) -> Result<usize> {
        let mut buffer = Vec::new();
        write!(
            buffer,
            "{} {} HTTP/1.1\r\n",
            request.method(),
            request.uri()
        )
        .unwrap();
        for (key, value) in request.headers() {
            write!(buffer, "{}: {}\r\n", key, value.to_str().unwrap()).unwrap();
        }
        write!(buffer, "\r\n").unwrap();
        buffer.extend(request.body().into_iter());

        // println!("{}", std::str::from_utf8(&buffer).unwrap());

        let size = stream.write(buffer.as_slice())?;
        stream.flush().unwrap();
        Ok(size)
    }

    pub fn get_available_bytes<T: Read + AsRawFd>(stream: &mut T) -> Result<usize> {
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

    fn parse_request_line(data: &[u8]) -> std::result::Result<(&str, &str, &str), Utf8Error> {
        let mut parts = data.split(|&b| b == b' ');

        let method = str::from_utf8(parts.next().unwrap_or(&[]))?;
        let path = str::from_utf8(parts.next().unwrap_or(&[]))?;
        let version = str::from_utf8(parts.next().unwrap_or(&[]))?;

        Ok((method, path, version))
    }

    fn parse_reponse_line(data: &[u8]) -> std::result::Result<(&str, &str, &str), Utf8Error> {
        let mut parts = data.split(|&b| b == b' ');

        let version = str::from_utf8(parts.next().unwrap_or(&[]))?;
        let code = str::from_utf8(parts.next().unwrap_or(&[]))?;
        let code_msg = str::from_utf8(parts.next().unwrap_or(&[]))?;

        Ok((version, code, code_msg))
    }

    fn parse_header(data: &[u8]) -> std::result::Result<(&str, &str), Utf8Error> {
        let mut parts = data.split(|&b| b == b':');

        let key = str::from_utf8(&parts.next().unwrap_or(&[]))?;
        let value = str::from_utf8(&parts.next().unwrap_or(&[])[1..])?;

        Ok((key, value))
    }

    pub fn read_request<T: Read + AsRawFd>(stream: &mut T) -> std::io::Result<Request<Vec<u8>>> {
        let size = get_available_bytes(stream)?;
        let mut buffer = vec![0u8; size];

        let size = stream.read(&mut buffer)?;

        let mut sequence = vec![(0usize, 0usize); 0];
        let buffer_iter = buffer.iter();
        let mut separator_buf = vec![0u8; 0];
        let mut has_body = (false, 0usize, 0usize);

        for (index, &value) in buffer_iter.enumerate() {
            match value {
                b'\n' => {
                    separator_buf.push(value);
                    if separator_buf.len() > 3 {
                        has_body = (true, index + 1, size);
                        break;
                    }
                }
                b'\r' => {
                    if separator_buf.len() == 0 {
                        if sequence.len() == 0 {
                            sequence.push((0, index));
                        } else {
                            sequence.push((sequence.last().unwrap().1 + 2, index));
                        }
                    }
                    separator_buf.push(value);
                }
                _ => {
                    separator_buf.clear();
                }
            }
        }

        let mut builder = request::Request::builder();

        let request_line = parse_request_line(&buffer[sequence[0].0..sequence[0].1]).unwrap();
        builder = builder
            .method(request_line.0)
            .uri(request_line.1)
            .version(|| -> Version {
                match Some(request_line.2) {
                    Some("HTTP/0.9") => Version::HTTP_09,
                    Some("HTTP/1.0") => Version::HTTP_09,
                    Some("HTTP/1.1") => Version::HTTP_09,
                    Some("HTTP/2.0") => Version::HTTP_09,
                    Some("HTTP/3.0") => Version::HTTP_09,
                    Some(_) | None => {
                        panic!("Unknown Http Version")
                    }
                }
            }());

        for (chunk_index, index) in sequence[1..].iter().enumerate() {
            let header = parse_header(&buffer[index.0..index.1]).unwrap();
            builder = builder.header(header.0, header.1);
        }

        if has_body.0 {
            let body = buffer[has_body.1..has_body.2].to_vec();
            let request = builder.body(body).unwrap();
            Ok(request)
        } else {
            let request = builder.body(vec![0; 0]).unwrap();
            Ok(request)
        }
    }

    pub fn read_response<T: Read + AsRawFd>(stream: &mut T) -> std::io::Result<Response<Vec<u8>>> {
        let size = get_available_bytes(stream)?;
        let mut buffer = vec![0u8; 1024];

        let size = stream.read(&mut buffer)?;

        let mut sequence = vec![(0usize, 0usize); 0];
        let buffer_iter = buffer.iter();
        let mut separator_buf = vec![0u8; 0];
        let mut has_body = (false, 0usize, 0usize);

        for (index, &value) in buffer_iter.enumerate() {
            match value {
                b'\n' => {
                    separator_buf.push(value);
                    if separator_buf.len() > 3 {
                        has_body = (true, index + 1, size);
                        break;
                    }
                }
                b'\r' => {
                    if separator_buf.len() == 0 {
                        if sequence.len() == 0 {
                            sequence.push((0, index));
                        } else {
                            sequence.push((sequence.last().unwrap().1 + 2, index));
                        }
                    }
                    separator_buf.push(value);
                }
                _ => {
                    separator_buf.clear();
                }
            }
        }


        let mut builder = response::Response::builder();

        let response_line = parse_reponse_line(&buffer[sequence[0].0..sequence[0].1]).unwrap();
        builder = builder.status(response_line.1).version(|| -> Version {
            match Some(response_line.0) {
                Some("HTTP/0.9") => Version::HTTP_09,
                Some("HTTP/1.0") => Version::HTTP_10,
                Some("HTTP/1.1") => Version::HTTP_11,
                Some("HTTP/2.0") => Version::HTTP_2,
                Some("HTTP/3.0") => Version::HTTP_3,
                Some(_) | None => {
                    panic!("Unknown Http Version")
                }
            }
        }());

        builder = builder.header("Sec-WebSocket-Accept", "tQiJ4LxcB6HBu3o3dm2i5qiU9js=");

        for (chunk_index, index) in sequence[1..].iter().enumerate() {
            let (header, header_val) = parse_header(&buffer[index.0..index.1]).unwrap();
            builder = builder.header(header, header_val);
        }

        if has_body.0 {
            let body = buffer[has_body.1..has_body.2].to_vec();
            let request = builder.body(body).unwrap();
            Ok(request)
        } else {
            let request = builder.body(vec![0; 0]).unwrap();
            Ok(request)
        }
    }

}
