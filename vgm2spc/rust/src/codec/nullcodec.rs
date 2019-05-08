///
/// A dummy codec that outputs the input data as-is.
///
 
use std::any::Any;
use std::vec::Vec;
use crate::codec::Codec;
use crate::bytestream::ByteStream;

pub struct NullCodec<'a> {
    output: &'a mut ByteStream,
}

impl<'a> Codec<'a> for NullCodec<'a> {
    fn new(out: &'a mut ByteStream) -> NullCodec<'a> {
	    NullCodec { output: out }	    
	}
    
	fn output_len(&self) -> usize {
		self.output.len()
	}
	
	fn passthrough(&mut self, c: u8) {
	    self.output.write(c);
	}
	
	fn write(&mut self, c: u8) {
	    self.passthrough(c);
	}
	
	fn flush(&mut self) {
	}

    fn get_extra_data(&self, what: u32) -> Option<Vec<u8>> {
		None
	}		
}

