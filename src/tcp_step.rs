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

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep};
    use crate::BoxedClone;

    pub struct TCPStep {
        tcp_stream: TcpStream,
        address: String,
    }

    impl PipelineStep for TCPStep {
        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };
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

        fn start(&self) {}
    }

    impl BoxedClone for TCPStep {
        fn bclone(&self) -> Box<dyn PipelineStep> {
            Box::new(TCPStep::new(&self.address))
        }
    }

    impl Read for TCPStep {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(self.tcp_stream.as_raw_fd(), libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else if available == 0 {
                Ok(0)
            } else {
                self.tcp_stream.read(buf)
            }
        }
    }

    impl Write for TCPStep {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.tcp_stream.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.tcp_stream.flush()
        }
    }

    #[allow(unreachable_code)]
    impl TCPStep {
        pub fn new(address: &str) -> Self {
            let mut connection: Option<TcpStream> = None;

            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            let port = uri.port().unwrap().as_u16();

            addr.push_str(":");
            addr.push_str(port.to_string().as_str());
            let connection = TcpStream::connect(addr).unwrap();

            //handle errors
            TCPStep {
                tcp_stream: connection,
                address: String::from_str(address).unwrap(),
            }
        }
    }
}