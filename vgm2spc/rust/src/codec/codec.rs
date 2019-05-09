use crate::bytestream::ByteStream;

pub trait Codec<'a> {
    fn new(output: &'a mut ByteStream) -> Self where Self: Sized;

    fn output_len(&self) -> usize;

    /// Add one byte of data without doing any processing on it.
    fn passthrough(&mut self, c: u8);

    /// Add one byte of data.
    fn write(&mut self, c: u8);

    /// Ensure that all data processed by the codec is written to its output.
    fn flush(&mut self);    

    fn get_extra_data(&self, what: u32) -> Option<Vec<u8>>;	
}