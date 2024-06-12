pub mod http_tools {
    use http::{request, response, version, Request, Response, Version};
    use rand::seq;
    use std::{
        io::{self, Read, Result, Write},
        iter::Iterator,
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

    fn get_available_bytes<T: Read + AsRawFd>(stream: &mut T) -> Result<usize> {
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

        stream.read(&mut buffer);

        let mut sequence = vec![0usize; 0];
        let buffer_iter = buffer.iter();
        let mut separator_buf = vec![0u8; 0];
        let mut has_body = (false, 0usize, 0usize);

        sequence.push(0);
        for (index, &value) in buffer_iter.enumerate() {
            match value {
                b'\n' => {
                    separator_buf.push(value);
                    if separator_buf.len() > 3 {
                        has_body = (true, index + 1, buffer.len());
                    }
                }
                b'\r' => {
                    if separator_buf.len() == 0 {
                        sequence.push(index);
                    }
                    separator_buf.push(value);
                }
                _ => {
                    separator_buf.clear();
                }
            }
        }

        let mut builder = request::Request::builder();

        let request_line = parse_request_line(&buffer[0..sequence[1]]).unwrap();
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
                Some(_)|
                None => {
                    panic!("Unknown Http Version")
                }
            }
        }());

        for (chunk_index, index) in sequence[1..]
            .windows(2)
            .map(|window| (window[0], window[1]))
            .enumerate()
        {
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

    pub fn read_response<T: Read + AsRawFd>(stream: &mut T) -> std::io::Result<Response<Vec<u8>>>
    {
        let size = get_available_bytes(stream)?;
        let mut buffer = vec![0u8; size];

        stream.read(&mut buffer);

        let mut sequence = vec![0usize; 0];
        let buffer_iter = buffer.iter();
        let mut separator_buf = vec![0u8; 0];
        let mut has_body = (false, 0usize, 0usize);

        sequence.push(0);
        for (index, &value) in buffer_iter.enumerate() {
            match value {
                b'\n' => {
                    separator_buf.push(value);
                    if separator_buf.len() > 3 {
                        has_body = (true, index + 1, buffer.len());
                    }
                }
                b'\r' => {
                    if separator_buf.len() == 0 {
                        sequence.push(index);
                    }
                    separator_buf.push(value);
                }
                _ => {
                    separator_buf.clear();
                }
            }
        }

        let mut builder = response::Response::builder();

        let response_line = parse_reponse_line(&buffer[0..sequence[1]]).unwrap();
        builder = builder
        .status(response_line.1)
        .version(|| -> Version {
            match Some(response_line.0) {
                Some("HTTP/0.9") => Version::HTTP_09,
                Some("HTTP/1.0") => Version::HTTP_09,
                Some("HTTP/1.1") => Version::HTTP_09,
                Some("HTTP/2.0") => Version::HTTP_09,
                Some("HTTP/3.0") => Version::HTTP_09,
                Some(_)|
                None => {
                    panic!("Unknown Http Version")
                }
            }
        }());

        for (chunk_index, index) in sequence[1..]
            .windows(2)
            .map(|window| (window[0], window[1]))
            .enumerate()
        {
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

    // use std::collections::HashMap;
    // use std::fmt::Display;
    // use std::hash::Hash;
    // use std::io::{self, Result, Write};

    // pub struct HttpMessage <T: AsRef<str> + Eq + PartialEq + Hash, B: AsRef<[u8]>>{
    //     headers: HashMap<T, T>,
    //     body: Option<B>
    // }

    // pub struct HttpMessageBuilder<T: AsRef<str> + Eq + PartialEq + Hash, B: AsRef<[u8]> + Iterator>{
    //     http_message: HttpMessage<T, B>
    // }

    // impl <T: AsRef<str> + Eq + PartialEq + Hash, B: AsRef<[u8]>+ Iterator> HttpMessage<T, B> {
    //     pub fn new() -> Self{
    //         HttpMessage::<T, B> { headers: HashMap::new(), body: None }
    //     }

    //     pub fn builder(self) -> HttpMessageBuilder<T, B> {
    //         HttpMessageBuilder::<T, B> {
    //             http_message: self
    //         }
    //     }

    // }

    // impl <T: AsRef<str> + Eq + PartialEq + Hash, B: AsRef<[u8]>+ Iterator> HttpMessageBuilder<T, B> {
    //     pub fn add_header(&mut self, key: T, value: T) -> &mut HttpMessageBuilder<T, B> {
    //         self.http_message.headers.insert(key, value);
    //         self
    //     }

    //     pub fn body(&mut self, data: B)-> &mut HttpMessageBuilder<T, B> {
    //         self.http_message.body = Some(data);
    //         self
    //     }

    //     pub fn build(self)-> HttpMessage<T, B> {
    //         self.http_message
    //     }
    // }
}
