pub mod http_tools {
    use http::{Request, Response};
    use std::{
        fmt::Display,
        io::{Read, Result, Write}, os::fd::AsRawFd,
    };

    pub fn write_response<T: Write, B: AsRef<[u8]>>(
        mut stream: T,
        response: Response<B>,
    ) -> Result<usize> {
        let mut buffer = Vec::new();

        // Serialize the request line
        write!(buffer, "HTTP/1.1 {}\r\n", response.status(),)?;

        // Serialize the headers
        for (key, value) in response.headers() {
            write!(buffer, "{}: {}\r\n", key, value.to_str().unwrap())?;
        }

        // End of headers
        write!(buffer, "\r\n")?;

        // Serialize the body
        buffer.write(response.body().as_ref())?;

        stream.write(buffer.as_ref())
    }

    fn get_available_bytes<T: Read + AsRawFd>(stream: T) -> Result<usize>{
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

    pub fn read_request<T: Read + AsRawFd, B: AsRef<[u8]>>(stream: T) -> Result<Request<B>> {
        let size = get_available_bytes(stream)?;
        let mut buffer = vec![0u8;size];

        if let Err(e) = stream.read(&mut buffer){
            return Err(e)
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
