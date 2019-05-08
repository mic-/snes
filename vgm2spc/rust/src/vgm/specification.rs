/// Partial enumeration of VGM commands (see https://vgmrips.net/wiki/VGM_Specification)
pub mod Command {
    pub const UNDEFINED: u8 = 0;          // not part of the VGM spec
    pub const NOP: u8 = 0x4E;             // not part of the VGM spec
	pub const GG_STEREO: u8 = 0x4F;
	pub const PSG_WRITE: u8 = 0x50;
	pub const YM2413_WRITE: u8 = 0x51;
	pub const YM2612_LO_WRITE: u8 = 0x52;
	pub const YM2612_HI_WRITE: u8 = 0x53;
	pub const YM2151_WRITE: u8 = 0x54;
	pub const WAIT_LONG: u8 = 0x61;
	pub const WAIT_NTSC_FRAME: u8 = 0x62;
	pub const WAIT_PAL_FRAME: u8 = 0x63;
	pub const END_OF_SOUND_DATA: u8 = 0x66;
	pub const DATA_BLOCK: u8 = 0x67;
	pub const PCM_WRITE: u8 = 0x68;
	pub const WAIT_1: u8 = 0x70;
	pub const WAIT_16: u8 = 0x7F;
	pub const YM2612_WRITE_LO_WAIT_0: u8 = 0x80; 
	pub const YM2612_WRITE_LO_WAIT_15: u8 = 0x8F;
    pub const WAIT_LONG_THRU_LUT: u8 = 0x90; // not part of the VGM spec
	pub const SEEK_PCM: u8 = 0xE0;
}

/// Returns the number of argument bytes expected by VGM command `cmd`.
pub fn num_argument_bytes(cmd: u8) -> u32 {
    match cmd {
        Command::GG_STEREO | Command::PSG_WRITE => 1,
        Command::YM2413_WRITE ... Command::YM2612_HI_WRITE => 2,
        Command::WAIT_LONG => 2,
        Command::SEEK_PCM => 4,
        _ => 0,
    }
}

pub const VGM_MAGIC: &'static str = "Vgm ";

#[repr(C, packed)]
pub struct FileHeader {
    pub magic: u32,
	pub eof_offset: u32,
	pub version: u32,
	pub psg_clock: u32,
	pub ym2413_clock: u32,
	pub gd3_offset: u32,
	pub total_samples: u32,
	pub loop_offset: u32,
	// 1.01
	pub rate: u32,
	// 1.10
	pub psg_feedback: u16,
	pub psg_lfsr_width: u8,
	pub psg_flags: u8,       // unused and assumed to be zero prior to 1.51
	pub ym2612_clock: u32,
	pub ym2151_clock: u32,
    // 1.50
	pub vgm_data_offset: u32
}