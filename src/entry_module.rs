pub mod entry_module {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };

    use crate::Pipeline;

    pub trait Entry : Clone {
        fn new(config: String, pipeline: Pipeline) -> Self;
        // fn len(&self) -> std::io::Result<usize>;
        // fn read(&mut self);
        // fn write(&mut self);
        fn len(&self, stream: &mut TcpStream) -> std::io::Result<usize>;
    }
}
