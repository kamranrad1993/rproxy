#[allow(non_snake_case, unused_variables, dead_code)]
pub mod tcp_step {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::str::FromStr;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::{Request, Uri};
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{client, Message, WebSocket};

    use crate::pipeline_module::pipeline::{IOError, PipelineDirection, PipelineStep};
    use crate::{BoxedClone,};

    pub struct TCPStep {
        tcp_stream: Option<TcpStream>,
        address: String,
    }

    impl PipelineStep for TCPStep {
        fn len(&mut self) -> std::io::Result<usize> {
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

        fn start(&mut self) {
            let mut connection: Option<TcpStream> = None;

            let uri: Uri = self.address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            let port = uri.port().unwrap().as_u16();

            addr.push_str(":");
            addr.push_str(port.to_string().as_str());
            let connection = TcpStream::connect(addr).unwrap();

            self.tcp_stream = Some(connection);
        }
    }

    impl BoxedClone for TCPStep {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(TCPStep::new(&self.address))
        }
    }

    impl crate::Read for TCPStep {
        fn read(&mut self) -> Result<Vec<u8>, IOError> {
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
                Err(IOError::IoError(errno))
            } else if available == 0 {
                Err(IOError::EmptyData)
            } else {
                let mut result = vec![0u8; available];
                self.get_stream().read(result.as_mut_slice());
                Ok(result)
            }
        }
    }

    impl Write for TCPStep {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.get_stream().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_stream().flush()
        }
    }

    #[allow(unreachable_code)]
    impl TCPStep {
        pub fn new(address: &str) -> Self {
            //handle errors
            TCPStep {
                tcp_stream: None,
                address: String::from_str(address).unwrap(),
            }
        }

        fn get_stream(&self) -> &TcpStream {
            self.tcp_stream.as_ref().unwrap()
        }
    }
}
