///
/// A VGM compressor focusing mainly on PSG commands (0x50 0xnn).
/// Each group of 8 commands is prepended with a flag byte, where each bit specifies
/// if the corresponding command is a PSG command or not. The command byte (0x50) is stripped
/// and only the argument byte is written to the output.
///
/// The compressor also tries to shorten long wait commands (0x61 0xmm 0xnn) down to one byte.
/// A table with 16 entries is filled with distinct wait lengths (0xnnmm) as they are found in
/// the VGM. A long wait command for which the length is found in the table is replaced by the
/// byte 0x9n, where n is the position in the table.
/// The table is stored in the output as a data block, right after the VGM header (i.e. offset 0x40).
///
/// Mic, 2010,2019
///

use std::any::Any;
use std::vec::Vec;
use crate::bytestream::ByteStream;
use crate::codec::Codec;
use crate::vgm::specification::Command;
use crate::vgm::specification::num_argument_bytes;

pub const GET_LONG_WAIT_LUT: u32 = 0;

pub struct PsgCodec<'a> {
	output: &'a mut ByteStream, // The codec's output data
	pending_data: Vec<u8>,      // Data that has been written to the codec but not yet been fully processed
    long_wait_table: Vec<u16>,  // A lookup table for compression of long wait VGM commands
    current_command: u8,
    remaning_argument_bytes: u32,
	long_wait_duration: u16,
	flags: u8,
    num_flags: u8,
}

impl<'a> PsgCodec<'a> {
    fn handle_argument(&mut self, arg: u8) {
        if self.current_command == Command::WAIT_LONG {
            let shifted_arg: u16 = (arg as u16) << ((2 - self.remaning_argument_bytes) * 8);
            self.long_wait_duration = self.long_wait_duration | shifted_arg;
            
            if self.remaning_argument_bytes == 1 {
                let pos = self.long_wait_table.iter().position(|&x| x == self.long_wait_duration);
                if pos.is_some() {
                    let idx: u8 = pos.unwrap() as u8;
                    self.pending_data.push(Command::WAIT_LONG_THRU_LUT | idx);
                } else if self.long_wait_table.len() < 16 {
                    // No match found, but there's space left in the LUT, so add the current value
                    self.pending_data.push(Command::WAIT_LONG_THRU_LUT | (self.long_wait_table.len() as u8));
                    self.long_wait_table.push(self.long_wait_duration);
                } else {
                    // No match could be found in the table. Store the entire command uncompressed.
                    self.pending_data.push(Command::WAIT_LONG);
                    self.pending_data.push((self.long_wait_duration & 0xFF) as u8);
                    self.pending_data.push((self.long_wait_duration >> 8) as u8);
                }
            }
        } else {
            self.pending_data.push(arg);
        }
        if self.remaning_argument_bytes > 0 {
            self.remaning_argument_bytes -= 1;
        }
        if self.remaning_argument_bytes == 0 {
            self.current_command = Command::UNDEFINED;
        }
    }
}

impl<'a> Codec<'a> for PsgCodec<'a> {
    fn new(out: &'a mut ByteStream) -> PsgCodec<'a> {
	    PsgCodec {
            output: out,
            pending_data: Vec::new(),
            long_wait_table: Vec::new(),
            current_command: Command::UNDEFINED,
            remaning_argument_bytes: 0,
            long_wait_duration: 0,
            flags: 0,
            num_flags: 0
        }	    
	}
    
	fn get_extra_data(&self, what: u32) -> Option<Vec<u8>> {
		match what {
			GET_LONG_WAIT_LUT => {
				let mut table = vec![Command::DATA_BLOCK, 0x66, 0x02, 0x20, 0x00, 0x00, 0x00];
				table.resize(32 + 7, 0);
				for (i, wait) in self.long_wait_table.iter().enumerate() {
					table[7 + i*2] = (wait & 0xFF) as u8;
					table[7 + i*2 + 1] = (wait >> 8) as u8;
				}
				Some(table)
			}
			_ => None
		}
	}
	
	fn output_len(&self) -> usize {
		self.output.len()
	}
	
	fn passthrough(&mut self, c: u8) {
	    self.output.write(c);
	}
	
	fn write(&mut self, c: u8) {
        if self.remaning_argument_bytes > 0 {
            self.handle_argument(c);
        } else {
            // New command
            if self.num_flags == 8 {
                self.flush();
            }
            
            self.current_command = c;
            self.remaning_argument_bytes = num_argument_bytes(self.current_command);
            
            match self.current_command {
                Command::PSG_WRITE => self.flags = self.flags | (1 << self.num_flags),
                Command::WAIT_LONG => self.long_wait_duration = 0,
                _ => self.pending_data.push(c),
            }
            self.num_flags += 1;
        }
    }

	fn flush(&mut self) {
		if self.num_flags > 0 {
			while self.num_flags < 8 {
				self.pending_data.push(Command::NOP);
				self.num_flags += 1;
			}
			self.output.write(self.flags);
			self.output.write_n(&self.pending_data);
			self.pending_data.clear();
			self.flags = 0;
			self.num_flags = 0;
		}	
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_psg() {
	    let mut bs = ByteStream::new(Vec::new());
        let mut codec = PsgCodec::new(&mut bs);
        assert_eq!(codec.flags, 0);
        codec.write(0x50);
        assert_eq!(codec.remaning_argument_bytes, 1);
        codec.write(0x12);
        assert_eq!(codec.output.len(), 0);
        assert_eq!(codec.pending_data.len(), 1);
        assert_eq!(codec.remaning_argument_bytes, 0);
        assert_eq!(codec.flags, 1);
        assert_eq!(codec.num_flags, 1);
    }

    #[test]
    fn test_write_long_wait() {
	    let mut bs = ByteStream::new(Vec::new());
        let mut codec = PsgCodec::new(&mut bs);
        codec.write(0x61);
        assert_eq!(codec.pending_data.len(), 0);
        assert_eq!(codec.long_wait_table.len(), 0);
        assert_eq!(codec.remaning_argument_bytes, 2);
        codec.write(0x12);
        codec.write(0x34);
        assert_eq!(codec.pending_data.len(), 1);
        assert_eq!(codec.long_wait_table.len(), 1);
        assert_eq!(codec.remaning_argument_bytes, 0);
        assert_eq!(codec.num_flags, 1);
    }
    
    #[test]
    fn test_implicit_flush() {
	    let mut bs = ByteStream::new(Vec::new());
        let mut codec = PsgCodec::new(&mut bs);
        assert_eq!(codec.flags, 0);
        for n in 0..8 {
            codec.write(0x50);
            codec.write(0x12);
        }
        codec.write(0x50);
        assert_eq!(codec.output.len(), 9);
        assert_eq!(codec.pending_data.len(), 0);
        assert_eq!(codec.flags, 1);
        assert_eq!(codec.num_flags, 1);
    }    
}
