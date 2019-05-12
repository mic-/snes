use std::io::{Error,ErrorKind};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use crate::bytestream::ByteStream;
use crate::codec::codec::Codec;
use crate::codec::nullcodec::NullCodec;
use crate::codec::psgcodec::PsgCodec;
use crate::codec::psgcodec;
use crate::vgm::specification::Command;
use crate::vgm::specification;
use crate::vgm::reader;

bitflags! {
    pub struct ConverterFlags: u32 {
        const NULL_CODEC = 0x00000000;
        const PSG_CODEC  = 0x00000001;
        const ASSUME_VGZ = 0x00000004;
        const RAW_OUTPUT = 0x00000008;
    }
}

pub struct Converter {
    loop_offset: usize,
    codec_used: ConverterFlags,
    song_title: String,
    game_title: String,
    artist: String,
}

impl Converter {
    pub fn new() -> Self {
        Converter {
            loop_offset: 0,
            codec_used: ConverterFlags::NULL_CODEC,
            song_title: String::from(""),
            game_title: String::from(""),
            artist: String::from(""),
        }
    }
    
    pub fn convert(&mut self, input_path: &Path, output_path: &Path, flags: ConverterFlags) -> Result<usize, std::io::Error> {
        self.codec_used = flags & (ConverterFlags::NULL_CODEC | ConverterFlags::PSG_CODEC);
        
        let mut input_data = Vec::new();
        reader::read_vgm_file(input_path, &mut input_data, flags.contains(ConverterFlags::ASSUME_VGZ))?;
        if input_data.len() < 32 {
            Error::new(ErrorKind::UnexpectedEof, "The file did not contain sufficient data");
        }
        let vgm_header: specification::FileHeader = unsafe { std::ptr::read(input_data.as_ptr() as *const _) };
        let data_offset = if vgm_header.version >= 0x00000150 {
            std::cmp::max(vgm_header.vgm_data_offset, 0x40)
        } else {
            0x40
        };
        
        let mut input_stream = ByteStream::new(input_data);
        let input_size = input_stream.len();
                
        println!("Converting {}", input_path.file_name().unwrap().to_str().unwrap());
        
        let extradata_offset = data_offset;
        let mut extradata_block: Vec<u8> = Vec::new();

        self.loop_offset = (vgm_header.loop_offset + 0x1C) as usize;
              
        input_stream = self.preprocess(&mut input_stream, data_offset as usize, &vgm_header);
        let mut output_stream = ByteStream::new(input_stream.read_n(data_offset as usize));
        output_stream.replace_at(8, 0x52);    // To identify the VGM as compressed

        let mut new_loop_offset = self.loop_offset;

        {
            // Now do the encoding stage
            let mut codec: Box<dyn Codec> = match self.codec_used {
                ConverterFlags::PSG_CODEC => Box::new(PsgCodec::new(&mut output_stream)),
                _ => Box::new(NullCodec::new(&mut output_stream)),
            };

            let mut eod = false;
            while !eod {
                if input_stream.get_pos() == vgm_header.loop_offset as usize {
                    codec.flush();
                    new_loop_offset = codec.output_len();
                }

                let c = input_stream.read();
                codec.write(c);

                match c {
                    Command::GG_STEREO | Command::PSG_WRITE | 0x30 => {
                        codec.write(input_stream.read());
                    }

                    Command::YM2413_WRITE ... Command::YM2151_WRITE => {
                        codec.write(input_stream.read());
                        codec.write(input_stream.read());
                    }

                    Command:: WAIT_LONG => {
                        codec.write(input_stream.read());
                        codec.write(input_stream.read());
                    }

                    Command::END_OF_SOUND_DATA => {
                        codec.flush();
                        eod = true;
                    }

                    Command::DATA_BLOCK => {
                        if input_stream.peek() == 0x66 {
                            let data_block_size = input_stream.peek_u32_at(2);
                            for _ in 0..data_block_size+6 {
                                codec.write(input_stream.read());
                            }
                        } else {
                            panic!("Illegal command: 0x67 0x{:X} at offset 0x{:X}", input_stream.peek(), input_stream.get_pos());
                        }
                    }

                    Command::SEEK_PCM => {
                        for _ in 0..4 { codec.write(input_stream.read()); }
                    }
                    _ => {}
                }
            }

            let long_wait_lut = codec.get_extra_data(psgcodec::GET_LONG_WAIT_LUT);
            if long_wait_lut.is_some() { extradata_block = long_wait_lut.unwrap(); }
        }

        let eof_offset = output_stream.len() + extradata_block.len() - 4;
        output_stream.replace_u32_at(4, eof_offset as u32);

        // Read rest of data, if any (GD3)
        if input_stream.available() > 0 {
            output_stream.write_n(&input_stream.read_available());
        }

        let mut gd3_offset = vgm_header.gd3_offset as usize;
        if gd3_offset != 0 {
            gd3_offset -= input_size - (output_stream.len() + extradata_block.len());
            output_stream.replace_u32_at(0x14, gd3_offset as u32);

            output_stream.skip(gd3_offset + 0x14 + 0x0C - extradata_block.len());
            let mut dummy = String::from("");
            Self::read_gd3_string(&mut output_stream, &mut self.song_title);
            Self::read_gd3_string(&mut output_stream, &mut dummy);  // Skip japanese title
            Self::read_gd3_string(&mut output_stream, &mut self.game_title);
            Self::read_gd3_string(&mut output_stream, &mut dummy);  // Skip japanese game title
            Self::read_gd3_string(&mut output_stream, &mut dummy);  // Skip system name
            Self::read_gd3_string(&mut output_stream, &mut dummy);  // Skip japanese system name
            Self::read_gd3_string(&mut output_stream, &mut self.artist);
            output_stream.reset();

            println!("Title: {}, Game: {}, Artist: {}", self.song_title, self.game_title, self.artist);	
        }

        if self.loop_offset > 0x1C {
            new_loop_offset += extradata_block.len();
            new_loop_offset -= 0x1C;
            output_stream.replace_u32_at(0x1C, new_loop_offset as u32);
        }

        println!("Input size: {} bytes, output size: {} bytes ({}%)", input_size, output_stream.len() + extradata_block.len(), 100 * (output_stream.len() + extradata_block.len()) / input_size);

        let mut player = match flags.contains(ConverterFlags::RAW_OUTPUT) {
            true => Vec::new(),
            false => Self::read_player_binary()?,
        };

        if (output_stream.len() + extradata_block.len()) > (0xFFC0 - player.len()) {
            Error::new(ErrorKind::InvalidInput, format!("The vgm data is too large to fit. The maximum size after packing is {} bytes", 0xFFC0 - player.len()));
        }

        let mut output_file = File::create(output_path)?;
        let mut spc_ram_remain = 0x10000;
        if !flags.contains(ConverterFlags::RAW_OUTPUT) {
            output_file.write_all("SNES-SPC700 Sound File Data v0.30".as_bytes())?;
            output_file.write_all(&[26, 26, 26, 30])?;

            // SPC registers:           PC       A     X     Y     PSW   SP     reserved
            output_file.write_all(&[0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?;
            // Write ID666 tag
            output_file.write_all(&Self::as_id666_buffer(self.song_title.as_bytes(), 32))?;
            output_file.write_all(&Self::as_id666_buffer(self.game_title.as_bytes(), 32))?;
            output_file.write_all(&Self::as_id666_buffer("Unknown".as_bytes(), 16))?;
            output_file.write_all(&Self::as_id666_buffer("Created with VGM2SPC".as_bytes(), 32))?;
            output_file.write_all(&Self::as_id666_buffer("01/01/1990".as_bytes(), 11))?;

            // Fade start/length (none)
            output_file.write_all(&vec![0; 8])?;

            output_file.write_all(&Self::as_id666_buffer(self.artist.as_bytes(), 32))?;
        
            // Channel disable (none), emulator used for dumping (unknown)
            output_file.write_all(&vec![0, 0])?;
            // Reserved
            output_file.write_all(&vec![0; 45])?;

            if player.len() >= 0xF0 { player[0xF0] = 0x0A; }    // SPC_TEST = 0x0A (enable timers, enable spc700)
            let player_bytes_used = std::cmp::min(player.len(), spc_ram_remain);
            output_file.write_all(&player[..player_bytes_used])?;
            spc_ram_remain -= player_bytes_used;
        }

        output_stream.reset();
        output_file.write_all(&output_stream.read_n(extradata_offset as usize))?;
        output_file.write_all(&extradata_block)?;
        output_file.write_all(&output_stream.read_available())?;
        spc_ram_remain -= output_stream.len() + extradata_block.len();

        if !flags.contains(ConverterFlags::RAW_OUTPUT) {
            // Pad SPC RAM block
            if spc_ram_remain > 0 {
                output_file.write_all(&vec![0; spc_ram_remain])?;
            }

            let mut dsp_regs: Vec<u8> = vec![0; 128];
            dsp_regs[0x6C] = 0x20;  // FLG = 0x20 (disable echo buffer writes);
            output_file.write_all(&dsp_regs[..])?;
            // Re-use as padding
            dsp_regs[0x6C] = 0;
            output_file.write_all(&dsp_regs[..])?;
        }

        Ok(0)
    }
    
    fn read_player_binary() -> Result<Vec<u8>, std::io::Error> {
        let mut player = Vec::new();
        let mut file = File::open("s-smp_player.bin")?;
        file.read_to_end(&mut player)?;
        Ok(player)
    }

    /// Return a vector of length `target_len` consisting of the data from `bytes`, plus as many padding zero-bytes as necessary
    fn as_id666_buffer(bytes: &[u8], target_len: usize) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        let bytes_used = std::cmp::min(bytes.len(), target_len);
        result.extend_from_slice(&bytes[..bytes_used]);
        if bytes_used < target_len { result.extend_from_slice(&vec![0; target_len - bytes_used]); }
        result
    }

    /// Read a GD3 tag string from the given stream, ignoring the high-order byte of each character
    fn read_gd3_string(bs: &mut ByteStream, str: &mut String) {
        for _ in 0..32 {
            if bs.available() == 0 { break; }
            let b = bs.peek();
            if b == 0 { break; }
            str.push(b as char);
            bs.skip(2);
        }
        while bs.available() != 0 && bs.peek() != 0 { bs.skip(2); }
        bs.skip(2);
    }

    fn preprocess(&mut self, input_stream: &mut ByteStream, starting_offset: usize, header: &specification::FileHeader) -> ByteStream {
        let mut preprocessed_data = ByteStream::new(input_stream.read_n(starting_offset));

        let mut ym_ch3_mode: u8 = 0;
        let mut last_pcm_offset: u32 = 0xFFFFFFFF;

        // Run a pre-processing stage to remove redundant commands
        let mut eod = false;
        while !eod {
            if input_stream.get_pos() == (header.loop_offset as usize) + 0x1C {
                self.loop_offset = preprocessed_data.len()
            }

            let c = input_stream.read();
            
            match c {
                Command::YM2612_LO_WRITE => {
                    let arg1 = input_stream.read();
                    if arg1 == 0x27 {
                        let arg2 = input_stream.read();
                        if (arg2 >> 6) != ym_ch3_mode {
                            ym_ch3_mode = arg2 >> 6;
                            preprocessed_data.write(c);
                            preprocessed_data.write(arg1);
                            preprocessed_data.write(arg2);
                        }
                    } else if arg1 == 0x25 || arg1 == 0x26 {
                        let _ = input_stream.read();
                    } else {
                        preprocessed_data.write(c);
                        preprocessed_data.write(arg1);
                        preprocessed_data.write(input_stream.read());
                    }
                }
                
                Command::END_OF_SOUND_DATA => {
                    preprocessed_data.write(c);
                    eod = true;
                }
                
                Command::DATA_BLOCK => {
                    if input_stream.peek() == 0x66 {
                        let data_block_size = input_stream.peek_u32_at(2);
                        preprocessed_data.write(c);
                        for _ in 0..data_block_size+6 {
                            preprocessed_data.write(input_stream.read());
                        }
                    } else {
                        panic!("Illegal command: 0x67 0x{:X} at offset 0x{:X}", input_stream.peek(), input_stream.get_pos());
                    }
                }

                Command::YM2612_WRITE_LO_WAIT_0 ... Command::YM2612_WRITE_LO_WAIT_15 => {
                    let mut wait = c & 0x0F;
                    // Merge as many adjacent 0x8n and 0x7n commands as possible into one 0x8n command
                    if (input_stream.peek() & 0xF0) == Command::WAIT_1 {
                        while (input_stream.peek() & 0xF0) == Command::WAIT_1 {
                            if (wait + (input_stream.peek() & 0x0F) + 1) < 0x10 {
                                wait += (input_stream.read() & 0x0F) + 1;
                            } else {
                                break;
                            }
                        }
                        preprocessed_data.write(0x80 | wait);
                    } else if preprocessed_data.len() > 0 {
                        let last = preprocessed_data.last().unwrap().clone();
                        if (last & 0xF0) == Command::WAIT_1 {
                            if (wait + (last & 0x0F) + 1) < 0x10 {
                                wait += (last & 0x0F) + 1;
                            }
                            let last_index = preprocessed_data.len() - 1;
                            preprocessed_data.reset();
                            preprocessed_data.skip(last_index);
                            preprocessed_data.replace_at(0, 0x80 | wait);
                            preprocessed_data.skip(1);
                        } else {
                            preprocessed_data.write(0x80 | wait);
                        }
                    } else {
                        preprocessed_data.write(0x80 | wait);
                    }
                }

                Command::SEEK_PCM => {
                    let pcm_offset = input_stream.peek_u32_at(0);
                    if pcm_offset != 0 && self.codec_used == ConverterFlags::NULL_CODEC {
                        preprocessed_data.write(c);
                        for _ in 0..4 {
                            preprocessed_data.write(input_stream.read());
                        }
                        last_pcm_offset = pcm_offset;
                    }
                }        

                _ => {
                    preprocessed_data.write(c);
                    for _ in 0..specification::num_argument_bytes(c) {
                        preprocessed_data.write(input_stream.read());
                    }
                }
            }
        }
        if input_stream.available() > 0 {
            preprocessed_data.write_n(&input_stream.read_available());
        }
        
        preprocessed_data
    }    
}
