pub mod entry_module {
    use std::{
        io::{Read, Write},
        net::TcpStream, os::fd::AsRawFd,
    };

    use crate::Pipeline;

    pub trait Entry : Clone {
        fn new(config: String, pipeline: Pipeline, loop_time: u64) -> Self;
        // fn len(&self) -> std::io::Result<usize>;
        // fn read(&mut self);
        // fn write(&mut self);
        fn len(stream: &mut dyn AsRawFd) -> std::io::Result<usize>;
        fn listen(&mut self);
    }
}
