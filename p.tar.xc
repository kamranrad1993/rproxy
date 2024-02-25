./src/                                                                                              0000775 0001750 0001750 00000000000 14566411134 011315  5                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 ./src/main.rs                                                                                       0000664 0001750 0001750 00000003502 14566562562 012623  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 use proxy::{Base64, Pipeline, PipelineStep, STDioStep, WebsocketDestination, WebsocketSource};
use std::time::Duration;

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -s define step           
  -h, --help     Print help

Steps:
  stdio:
  ws://
  b64:fw b64:bw
";

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", USAGE);
        std::process::exit(0);
    }

    let mut forward_steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    loop {
        let step = pargs.opt_value_from_str::<&str, String>("-s").unwrap();
        if step == None {
            break;
        }
        let step = step.unwrap();
        println!("{step}");

        let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
        let protocol = Some(res.get(0).unwrap().as_str());
        let config = Some(res.get(1).unwrap().as_str());
        match protocol {
            Some("stdio") => {
                forward_steps.push(Box::new(STDioStep::new()));
            }
            Some("ws-l") => forward_steps.push(Box::new(WebsocketSource::new(step.as_str()))),
            Some("ws") => forward_steps.push(Box::new(WebsocketDestination::new(step.as_str()))),
            Some("b64") => forward_steps.push(Box::new(Base64::new(config))),
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    let mut pipeline = Pipeline::new(forward_steps, Some(1024)).unwrap();
    #[allow(while_true)]
    while true {
        pipeline.read_source().unwrap();
        pipeline.read_destination().unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
}
                                                                                                                                                                                              ./src/stdio_step.rs                                                                                 0000664 0001750 0001750 00000003550 14566562700 014051  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 pub mod io_step {
    use crate::pipeline_module::pipeline::{PipelineStep, PipelineStepType, PipelineDirection};
    use std::io::{stdin, stdout, Read, Write};

    pub struct STDioStep {}

    impl PipelineStep for STDioStep {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::SourceAndDest
        }

        fn len(&self) -> std::io::Result<usize> {
            let mut available: usize = 0;
            let result: i32 =
                unsafe { libc::ioctl(0, libc::FIONREAD, &mut available) };
            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            }else {
                Ok(available)
            }
        }

        #[allow(unused_variables)]
        fn set_pipeline_direction (&mut self, direction: PipelineDirection){
            
        }
    }

    impl Default for STDioStep {
        fn default() -> Self {
            Self {}
        }
    }

    impl Read for STDioStep {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut io = stdin();
            let mut available: usize = 0;
            let result: i32 = unsafe { libc::ioctl(0, libc::FIONREAD, &mut available) };

            if result == -1 {
                let errno = std::io::Error::last_os_error();
                Err(errno)
            } else if available == 0 {
                Ok(0)
            } else {
                io.read(buf)
            }
        }
    }

    impl Write for STDioStep {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut io = stdout();
            io.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut io = stdout();
            io.flush()
        }
    }

    impl STDioStep {
        pub fn new() -> STDioStep {
            STDioStep {}
        }
    }
}
                                                                                                                                                        ./src/websocket_step.rs                                                                             0000664 0001750 0001750 00000021670 14566562645 014730  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 #[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_source {
    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        os::fd::AsRawFd,
    };
    use tungstenite::http::Uri;
    use tungstenite::{accept, protocol::Role, Message, WebSocket};

    pub struct WebsocketSource {
        _tcp_server: TcpListener,
        tcp_stream: TcpStream,
    }

    impl Write for WebsocketSource {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            self.get_websocket().send(msg).unwrap();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
            // self.tcp_stream.flush()
        }
    }

    impl Read for WebsocketSource {
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
                // self.tcp_stream.read(buf)
                let m = &mut self.get_websocket().read().unwrap();
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match m {
                    Message::Text(data) => {
                        unsafe {
                            let length = std::cmp::min(data.as_bytes().len(), buf.len());
                            std::ptr::copy(
                                data.as_mut_ptr(),
                                buf.as_mut_ptr(),
                                length,
                            );
                            Ok(length)
                        }
                        // buf.copy_from_slice(data.as_bytes());
                    }
                    Message::Binary(data) => {
                        unsafe {
                            let length = std::cmp::min(data.len(), buf.len());
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), length);
                            Ok(length)
                        }
                        // buf.copy_from_slice(data.as_slice());
                        // buf = data.as_mut_slice();
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                        Ok(0)
                    }
                }
            }
        }
    }

    impl PipelineStep for WebsocketSource {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Source
        }

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
    }

    impl WebsocketSource {
        pub fn new(address: &str) -> Self {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let server = TcpListener::bind(addr).unwrap();
            let r = server.accept().unwrap().0;
            accept(r.try_clone().unwrap()).unwrap();
            WebsocketSource {
                _tcp_server: server,
                tcp_stream: r, // websocket: websocket,
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Server, None)
        }
    }
}

#[allow(non_snake_case, unused_variables, dead_code)]
pub mod ws_destination {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use tungstenite::client::client;
    use tungstenite::client::IntoClientRequest;
    use tungstenite::http::Uri;
    use tungstenite::protocol::{Role, WebSocketContext};
    use tungstenite::{Message, WebSocket};

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};

    pub struct WebsocketDestination {
        tcp_stream: TcpStream,
        context: WebSocketContext,
    }

    impl PipelineStep for WebsocketDestination {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Destination
        }

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
    }

    impl Read for WebsocketDestination {
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
                let m = &mut self.get_websocket().read().unwrap();
                // let mut m = &mut self
                //     .context
                //     .read::<TcpStream>(&mut self.tcp_stream)
                //     .unwrap();
                match m {
                    Message::Text(data) => {
                        unsafe {
                            let length = std::cmp::min(data.as_bytes().len(), buf.len());
                            std::ptr::copy(
                                data.as_mut_ptr(),
                                buf.as_mut_ptr(),
                                data.as_bytes().len(),
                            );
                            Ok(length)
                        }
                    }
                    Message::Binary(data) => {
                        unsafe {
                            let length = std::cmp::min(data.len(), buf.len());
                            std::ptr::copy(data.as_mut_ptr(), buf.as_mut_ptr(), data.len());
                            Ok(length)
                        }
                    }
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
                        Ok(0)
                    }
                }
                // self.tcp_stream.read(buf)
            }
        }
    }

    impl Write for WebsocketDestination {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let vec = Vec::from(buf);
            let msg = Message::Binary(vec);
            let result = self.get_websocket().send(msg);
            match result {
                Ok(_) => Ok(buf.len()),
                Err(error) => {
                    panic!("{}", error);
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.get_websocket().flush().unwrap();
            Ok(())
        }
    }

    impl WebsocketDestination {
        pub fn new(address: &str) -> Self {
            let uri: Uri = address.parse::<Uri>().unwrap();
            let mut addr = String::from(uri.host().unwrap());
            addr.push_str(":");
            addr.push_str(uri.port().unwrap().as_str());
            let connection = TcpStream::connect(addr).unwrap();
            // let req = Request::builder().uri(address).body(()).unwrap();
            let req: tungstenite::http::Request<()> = uri.into_client_request().unwrap();
            // let l = client_with_config(req, connection.try_clone().unwrap(), None).unwrap();
            client(req, connection.try_clone().unwrap()).unwrap();

            //handle errors
            WebsocketDestination {
                tcp_stream: connection,
                context: WebSocketContext::new(Role::Client, None),
            }
        }

        pub fn get_websocket(&self) -> WebSocket<TcpStream> {
            WebSocket::from_raw_socket(self.tcp_stream.try_clone().unwrap(), Role::Client, None)
        }
    }
}
                                                                        ./src/lib.rs                                                                                        0000664 0001750 0001750 00000000534 14566411134 012433  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 mod pipeline_module;
pub use pipeline_module::{pipeline::Pipeline, pipeline::PipelineStep, pipeline::PipelineDirection};

mod websocket_step;
pub use websocket_step::{ws_destination::WebsocketDestination, ws_source::WebsocketSource};

mod stdio_step;
pub use stdio_step::io_step::STDioStep;

mod base64_step;
pub use base64_step::{base64::Base64};
                                                                                                                                                                    ./src/pipeline_module.rs                                                                            0000664 0001750 0001750 00000015164 14566561337 015057  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 // #[allow(non_snake_case, unused_variables, dead_code)]

pub mod pipeline {
    use std::{
        fmt::Display,
        io::{self, Read, Write},
        ops::{BitAnd, Deref, DerefMut},
        string::ParseError,
    };

    #[derive(Debug)]
    pub enum IOError {
        InvalidConnection,
        InvalidBindAddress,
        UnknownError(String),
        IoError(io::Error),
        ParseError(ParseError),
        InvalidStep(String),
    }

    #[repr(u32)]
    #[derive(Copy, Clone, Eq, Debug)]
    pub enum PipelineStepType {
        Source = 0x01,
        Middle = 0x02,
        Destination = 0x04,
        SourceAndDest = 0x05,
    }

    pub enum PipelineDirection {
        Forward = 0x01,
        Backward = 0x02,
    }

    impl Clone for PipelineDirection {
        fn clone(&self) -> Self {
            match self {
                Self::Forward => Self::Forward,
                Self::Backward => Self::Backward,
            }
        }
    }

    impl Copy for PipelineDirection {}

    impl PartialEq for PipelineDirection {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    impl Display for PipelineDirection {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PipelineDirection::Forward => write!(f, "PipelineDirection::Forward"),
                PipelineDirection::Backward => write!(f, "PipelineDirection::Backward"),
            }
        }
    }

    impl BitAnd for PipelineStepType {
        type Output = PipelineStepType;

        fn bitand(self, rhs: Self) -> Self::Output {
            let result_u32 = self as u32 & rhs as u32;
            match result_u32 {
                0x01 => PipelineStepType::Source,
                0x02 => PipelineStepType::Middle,
                0x04 => PipelineStepType::Destination,
                0x05 => PipelineStepType::SourceAndDest,
                _ => panic!("Invalid combination of flags"),
            }
        }
    }

    impl PartialEq for PipelineStepType {
        fn eq(&self, other: &Self) -> bool {
            core::mem::discriminant(self) == core::mem::discriminant(other)
        }
    }

    impl Display for PipelineStepType{
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PipelineStepType::Source => write!(f, "PipelineStepType::Source"),
                PipelineStepType::Middle => write!(f, "PipelineStepType::Middle"),
                PipelineStepType::Destination => write!(f, "PipelineStepType::Destination"),
                PipelineStepType::SourceAndDest => write!(f, "PipelineStepType::SourceAndDest"),
            }
        }
    }

    pub trait PipelineStep: Read + Write + Send + Sync {
        fn get_step_type(&self) -> PipelineStepType;
        fn len(&self) -> std::io::Result<usize>;
        fn set_pipeline_direction(&mut self, direction: PipelineDirection);
    }

    pub struct Pipeline {
        steps: Vec<Box<dyn PipelineStep>>,
        buffer_size: Option<usize>,
    }

    impl Deref for Pipeline {
        type Target = [Box<dyn PipelineStep>];

        fn deref(&self) -> &Self::Target {
            &self.steps
        }
    }

    impl DerefMut for Pipeline {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.steps
        }
    }

    impl Pipeline {
        pub fn new(
            steps: Vec<Box<dyn PipelineStep>>,
            buffer_size: Option<usize>,
        ) -> Result<Self, IOError> {
            if steps.len() < 2 {
                Err(IOError::InvalidStep(format!(
                    "Step count must greater than two."
                )))
            } else if steps.first().unwrap().get_step_type() & PipelineStepType::Source
                != PipelineStepType::Source
            {
                Err(IOError::InvalidStep(format!(
                    "First step type must be PipelineStepType::Source."
                )))
            } else if steps.last().unwrap().get_step_type() & PipelineStepType::Destination
                != PipelineStepType::Destination
            {
                Err(IOError::InvalidStep(format!(
                    "Last step type must be PipelineStepType::Destination."
                )))
            } else {
                for i in 1..steps.len() - 2 {
                    if steps[i].get_step_type() & PipelineStepType::Middle
                        != PipelineStepType::Middle
                    {
                        return Err(IOError::InvalidStep(format!(
                            "Middle step type must be PipelineStepType::Middle."
                        )));
                    }
                }

                Ok(Pipeline {
                    steps: steps,
                    buffer_size: Some(buffer_size.unwrap_or(1024)),
                })
            }
        }

        pub fn read_source(&mut self) -> Result<(), IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Forward);
                // println!("{}", self.steps[i].get_step_type());
            }

            for i in 0..self.steps.len() - 1 {
                let mut size = std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0; size];
                // self.steps[i].set_pipeline_direction(PipelineDirection::Forward);
                size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    // self.steps[i + 1].set_pipeline_direction(PipelineDirection::Forward);
                    self.steps[i + 1].write(&data[0..size]).unwrap();
                    self.steps[i + 1].flush().unwrap();
                }
            }
            Ok(())
        }

        pub fn read_destination(&mut self) -> Result<(), IOError> {
            for i in 0..self.steps.len() {
                self.steps[i].set_pipeline_direction(PipelineDirection::Backward);
            }

            for i in (1..self.steps.len()).rev() {
                let mut size = std::cmp::min(self.buffer_size.unwrap(), self.steps[i].len().unwrap());
                let mut data: Vec<u8> = vec![0; size];
                // self.steps[i].set_pipeline_direction(PipelineDirection::Backward);
                size = self.steps[i].read(data.as_mut_slice()).unwrap();
                if size > 0 {
                    // self.steps[i - 1].set_pipeline_direction(PipelineDirection::Backward);
                    self.steps[i - 1].write(&data[0..size]).unwrap();
                    self.steps[i - 1].flush().unwrap();
                }
            }
            Ok(())
        }
    }
}
                                                                                                                                                                                                                                                                                                                                                                                                            ./src/base64_step.rs                                                                                0000664 0001750 0001750 00000010467 14566561665 014031  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 #[allow(noop_method_call, unused_assignments)]
pub mod base64 {

    use crate::pipeline_module::pipeline::{PipelineDirection, PipelineStep, PipelineStepType};
    use base64::{
        alphabet,
        engine::{
            general_purpose::NO_PAD,
            GeneralPurpose,
        },
        Engine as _,
    };
    use std::io::{Read, Write};

    pub const B64_ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::BIN_HEX, NO_PAD);
    pub const NEW_LINE: &[u8] = &[b'\n'; 1];
    pub struct Base64 {
        forward_buffer: Vec<u8>,
        backward_buffer: Vec<u8>,
        work_mode: PipelineDirection,
        pipeline_direction: PipelineDirection,
    }

    impl PipelineStep for Base64 {
        fn get_step_type(&self) -> PipelineStepType {
            PipelineStepType::Middle
        }

        fn len(&self) -> std::io::Result<usize> {
            // Ok(self.forward_buffer.len())
            match self.pipeline_direction {
                PipelineDirection::Forward => Ok(self.forward_buffer.len()),
                PipelineDirection::Backward => Ok(self.backward_buffer.len()),
            }
        }

        fn set_pipeline_direction(&mut self, direction: PipelineDirection) {
            self.pipeline_direction = direction;
        }
    }

    impl Read for Base64 {
        fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    let length = std::cmp::min(self.forward_buffer.len(), buf.len());
                    let size = buf.write(&self.forward_buffer[0..length]).unwrap();
                    self.forward_buffer.drain(0..size);
                    Ok(size)
                }
                PipelineDirection::Backward => {
                    let length = std::cmp::min(self.backward_buffer.len(), buf.len());
                    let size = buf.write(&self.backward_buffer[0..length]).unwrap();
                    self.backward_buffer.drain(0..size);
                    Ok(size)
                }
            }
        }
    }

    impl Write for Base64 {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut local_buf = buf.clone();
            if buf.ends_with(NEW_LINE) {
                local_buf = &buf[0..buf.len() - 1];
            }
            match self.pipeline_direction {
                PipelineDirection::Forward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64_ENGINE.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.forward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let b64 = B64_ENGINE.decode(local_buf).unwrap();
                        self.forward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
                PipelineDirection::Backward => {
                    if self.work_mode == self.pipeline_direction {
                        let b64 = B64_ENGINE.encode(local_buf);
                        let data: &[u8] = b64.as_bytes();
                        self.backward_buffer.extend(data);
                        Ok(data.len())
                    } else {
                        let s = std::str::from_utf8(local_buf).unwrap();
                        let b64 = B64_ENGINE.decode(s).unwrap();
                        self.backward_buffer.extend(b64.as_slice());
                        Ok(b64.len())
                    }
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Base64 {
        pub fn new(config: Option<&str>) -> Base64 {
            let mut work_mode = PipelineDirection::Forward;
            match config {
                Some("fw") => work_mode = PipelineDirection::Forward,
                Some("bw") => work_mode = PipelineDirection::Backward,
                Some(_) | None => {
                    panic!("Base64Encoder : Unknown Work Mode")
                }
            }
            Base64 {
                forward_buffer: vec![0; 0],
                backward_buffer: vec![0; 0],
                work_mode: work_mode,
                pipeline_direction: PipelineDirection::Forward,
            }
        }
    }
}
                                                                                                                                                                                                         ./Cargo.toml                                                                                        0000664 0001750 0001750 00000000710 14566560661 012466  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 [package]
name = "proxy"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
async-trait = "0.1.77"
pico-args = { version = "0.5.0", features = ["combined-flags","eq-separator","short-space-opt"] }
libc = "0.2.153"
tungstenite = "0.21.0"
strum = { version = "0.26.1", features = [
    "std",
    "derive",
    "strum_macros",
    "phf",
] }
base64 = "0.21.7"
                                                        ./Cargo.lock                                                                                        0000664 0001750 0001750 00000030076 14566560652 012453  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 # This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 3

[[package]]
name = "async-trait"
version = "0.1.77"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c980ee35e870bd1a4d2c8294d4c04d0499e67bca1e4b5cefcc693c2fa00caea9"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.48",
]

[[package]]
name = "base64"
version = "0.21.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9d297deb1925b89f2ccc13d7635fa0714f12c87adce1c75356b39ca9b7178567"

[[package]]
name = "block-buffer"
version = "0.10.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71"
dependencies = [
 "generic-array",
]

[[package]]
name = "byteorder"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fd0f2584146f6f2ef48085050886acf353beff7305ebd1ae69500e27c67f64b"

[[package]]
name = "bytes"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a2bd12c1caf447e69cd4528f47f94d203fd2582878ecb9e9465484c4148a8223"

[[package]]
name = "cfg-if"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "baf1de4339761588bc0619e3cbc0120ee582ebb74b53b4efbf79117bd2da40fd"

[[package]]
name = "cpufeatures"
version = "0.2.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "53fe5e26ff1b7aef8bca9c6080520cfb8d9333c7568e1829cef191a9723e5504"
dependencies = [
 "libc",
]

[[package]]
name = "crypto-common"
version = "0.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1bfb12502f3fc46cca1bb51ac28df9d618d813cdc3d2f25b9fe775a34af26bb3"
dependencies = [
 "generic-array",
 "typenum",
]

[[package]]
name = "data-encoding"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7e962a19be5cfc3f3bf6dd8f61eb50107f356ad6270fbb3ed41476571db78be5"

[[package]]
name = "digest"
version = "0.10.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292"
dependencies = [
 "block-buffer",
 "crypto-common",
]

[[package]]
name = "fnv"
version = "1.0.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3f9eec918d3f24069decb9af1554cad7c880e2da24a9afd88aca000531ab82c1"

[[package]]
name = "form_urlencoded"
version = "1.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e13624c2627564efccf4934284bdd98cbaa14e79b0b5a141218e507b3a823456"
dependencies = [
 "percent-encoding",
]

[[package]]
name = "generic-array"
version = "0.14.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a"
dependencies = [
 "typenum",
 "version_check",
]

[[package]]
name = "getrandom"
version = "0.2.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "190092ea657667030ac6a35e305e62fc4dd69fd98ac98631e5d3a2b1575a12b5"
dependencies = [
 "cfg-if",
 "libc",
 "wasi",
]

[[package]]
name = "heck"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "95505c38b4572b2d910cecb0281560f54b440a19336cbbcb27bf6ce6adc6f5a8"

[[package]]
name = "http"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b32afd38673a8016f7c9ae69e5af41a58f81b1d31689040f2f1959594ce194ea"
dependencies = [
 "bytes",
 "fnv",
 "itoa",
]

[[package]]
name = "httparse"
version = "1.8.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d897f394bad6a705d5f4104762e116a75639e470d80901eed05a860a95cb1904"

[[package]]
name = "idna"
version = "0.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "634d9b1461af396cad843f47fdba5597a4f9e6ddd4bfb6ff5d85028c25cb12f6"
dependencies = [
 "unicode-bidi",
 "unicode-normalization",
]

[[package]]
name = "itoa"
version = "1.0.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b1a46d1a171d865aa5f83f92695765caa047a9b4cbae2cbf37dbd613a793fd4c"

[[package]]
name = "libc"
version = "0.2.153"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9c198f91728a82281a64e1f4f9eeb25d82cb32a5de251c6bd1b5154d63a8e7bd"

[[package]]
name = "log"
version = "0.4.20"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b5e6163cb8c49088c2c36f57875e58ccd8c87c7427f7fbd50ea6710b2f3f2e8f"

[[package]]
name = "percent-encoding"
version = "2.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e3148f5046208a5d56bcfc03053e3ca6334e51da8dfb19b6cdc8b306fae3283e"

[[package]]
name = "phf"
version = "0.10.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fabbf1ead8a5bcbc20f5f8b939ee3f5b0f6f281b6ad3468b84656b658b455259"
dependencies = [
 "phf_macros",
 "phf_shared",
 "proc-macro-hack",
]

[[package]]
name = "phf_generator"
version = "0.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5d5285893bb5eb82e6aaf5d59ee909a06a16737a8970984dd7746ba9283498d6"
dependencies = [
 "phf_shared",
 "rand",
]

[[package]]
name = "phf_macros"
version = "0.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "58fdf3184dd560f160dd73922bea2d5cd6e8f064bf4b13110abd81b03697b4e0"
dependencies = [
 "phf_generator",
 "phf_shared",
 "proc-macro-hack",
 "proc-macro2",
 "quote",
 "syn 1.0.109",
]

[[package]]
name = "phf_shared"
version = "0.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b6796ad771acdc0123d2a88dc428b5e38ef24456743ddb1744ed628f9815c096"
dependencies = [
 "siphasher",
]

[[package]]
name = "pico-args"
version = "0.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5be167a7af36ee22fe3115051bc51f6e6c7054c9348e28deb4f49bd6f705a315"

[[package]]
name = "ppv-lite86"
version = "0.2.17"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5b40af805b3121feab8a3c29f04d8ad262fa8e0561883e7653e024ae4479e6de"

[[package]]
name = "proc-macro-hack"
version = "0.5.20+deprecated"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "dc375e1527247fe1a97d8b7156678dfe7c1af2fc075c9a4db3690ecd2a148068"

[[package]]
name = "proc-macro2"
version = "1.0.78"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e2422ad645d89c99f8f3e6b88a9fdeca7fabeac836b1002371c4367c8f984aae"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "proxy"
version = "0.1.0"
dependencies = [
 "async-trait",
 "base64",
 "libc",
 "pico-args",
 "strum",
 "tungstenite",
]

[[package]]
name = "quote"
version = "1.0.35"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "291ec9ab5efd934aaf503a6466c5d5251535d108ee747472c3977cc5acc868ef"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "rand"
version = "0.8.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "34af8d1a0e25924bc5b7c43c079c942339d8f0a8b57c39049bef581b46327404"
dependencies = [
 "libc",
 "rand_chacha",
 "rand_core",
]

[[package]]
name = "rand_chacha"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e6c10a63a0fa32252be49d21e7709d4d4baf8d231c2dbce1eaa8141b9b127d88"
dependencies = [
 "ppv-lite86",
 "rand_core",
]

[[package]]
name = "rand_core"
version = "0.6.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ec0be4795e2f6a28069bec0b5ff3e2ac9bafc99e6a9a7dc3547996c5c816922c"
dependencies = [
 "getrandom",
]

[[package]]
name = "rustversion"
version = "1.0.14"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7ffc183a10b4478d04cbbbfc96d0873219d962dd5accaff2ffbd4ceb7df837f4"

[[package]]
name = "sha1"
version = "0.10.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e3bf829a2d51ab4a5ddf1352d8470c140cadc8301b2ae1789db023f01cedd6ba"
dependencies = [
 "cfg-if",
 "cpufeatures",
 "digest",
]

[[package]]
name = "siphasher"
version = "0.3.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "38b58827f4464d87d377d175e90bf58eb00fd8716ff0a62f80356b5e61555d0d"

[[package]]
name = "strum"
version = "0.26.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "723b93e8addf9aa965ebe2d11da6d7540fa2283fcea14b3371ff055f7ba13f5f"
dependencies = [
 "phf",
 "strum_macros",
]

[[package]]
name = "strum_macros"
version = "0.26.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7a3417fc93d76740d974a01654a09777cb500428cc874ca9f45edfe0c4d4cd18"
dependencies = [
 "heck",
 "proc-macro2",
 "quote",
 "rustversion",
 "syn 2.0.48",
]

[[package]]
name = "syn"
version = "1.0.109"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "72b64191b275b66ffe2469e8af2c1cfe3bafa67b529ead792a6d0160888b4237"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "syn"
version = "2.0.48"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0f3531638e407dfc0814761abb7c00a5b54992b849452a0646b7f65c9f770f3f"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "thiserror"
version = "1.0.56"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d54378c645627613241d077a3a79db965db602882668f9136ac42af9ecb730ad"
dependencies = [
 "thiserror-impl",
]

[[package]]
name = "thiserror-impl"
version = "1.0.56"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fa0faa943b50f3db30a20aa7e265dbc66076993efed8463e8de414e5d06d3471"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.48",
]

[[package]]
name = "tinyvec"
version = "1.6.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "87cc5ceb3875bb20c2890005a4e226a4651264a5c75edb2421b52861a0a0cb50"
dependencies = [
 "tinyvec_macros",
]

[[package]]
name = "tinyvec_macros"
version = "0.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1f3ccbac311fea05f86f61904b462b55fb3df8837a366dfc601a0161d0532f20"

[[package]]
name = "tungstenite"
version = "0.21.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9ef1a641ea34f399a848dea702823bbecfb4c486f911735368f1f137cb8257e1"
dependencies = [
 "byteorder",
 "bytes",
 "data-encoding",
 "http",
 "httparse",
 "log",
 "rand",
 "sha1",
 "thiserror",
 "url",
 "utf-8",
]

[[package]]
name = "typenum"
version = "1.17.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "42ff0bf0c66b8238c6f3b578df37d0b7848e55df8577b3f74f92a69acceeb825"

[[package]]
name = "unicode-bidi"
version = "0.3.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "08f95100a766bf4f8f28f90d77e0a5461bbdb219042e7679bebe79004fed8d75"

[[package]]
name = "unicode-ident"
version = "1.0.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3354b9ac3fae1ff6755cb6db53683adb661634f67557942dea4facebec0fee4b"

[[package]]
name = "unicode-normalization"
version = "0.1.22"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5c5713f0fc4b5db668a2ac63cdb7bb4469d8c9fed047b1d0292cc7b0ce2ba921"
dependencies = [
 "tinyvec",
]

[[package]]
name = "url"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "31e6302e3bb753d46e83516cae55ae196fc0c309407cf11ab35cc51a4c2a4633"
dependencies = [
 "form_urlencoded",
 "idna",
 "percent-encoding",
]

[[package]]
name = "utf-8"
version = "0.7.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "09cc8ee72d2a9becf2f2febe0205bbed8fc6615b7cb429ad062dc7b7ddd036a9"

[[package]]
name = "version_check"
version = "0.9.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "49874b5167b65d7193b8aba1567f5c7d93d001cafc34600cee003eda787e483f"

[[package]]
name = "wasi"
version = "0.11.0+wasi-snapshot-preview1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9c8d87e72b64a3b4db28d11ce29237c246188f4f51057d65a7eab63b7987e423"
                                                                                                                                                                                                                                                                                                                                                                                                                                                                  ./tools/                                                                                            0000775 0001750 0001750 00000000000 14564164476 011702  5                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 ./tools/echo.py                                                                                     0000664 0001750 0001750 00000000645 14564164476 013177  0                                                                                                    ustar   kamran                          kamran                                                                                                                                                                                                                 from simple_websocket_server import WebSocketServer, WebSocket


class SimpleEcho(WebSocket):
    def handle(self):
        # echo message back to client
        print(self.data)
        self.send_message(self.data)

    def connected(self):
        print(self.address, 'connected')

    def handle_close(self):
        print(self.address, 'closed')


server = WebSocketServer('', 6666, SimpleEcho)
server.serve_forever()                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           