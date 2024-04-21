pub mod websocket_entry {
    use regex::Regex;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
    };

    use crate::Entry;

    pub struct WebsocketEntry {
        _tcp_server: TcpListener,
        tcp_stream: TcpStream,
        address: String,
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
        fn new(config : String, pipeline: crate::Pipeline) -> Self {
            
        }

        fn len(&self) -> std::io::Result<usize> {
            todo!()
        }
    }
}
