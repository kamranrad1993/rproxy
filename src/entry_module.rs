pub mod entry_module {
    use std::io::{Read, Write};

    use crate::Pipeline;

    pub trait Entry: Read + Write {
        fn new(config: String, pipeline: Pipeline) -> Self;
        fn len(&self) -> std::io::Result<usize>;
    }
}
