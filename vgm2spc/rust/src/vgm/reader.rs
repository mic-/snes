use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use flate2::read::GzDecoder;
use crate::vgm::specification;

/// Reads the VGM file given by `input_path` into the vector `out_data`.
///
/// Both compressed (VGZ) and uncompressed VGM files are supported.
/// The flag `assume_vgz` can be used to force the file to be treated as compressed. Otherwise the
/// function will try to detect the compression by itself.
pub fn read_vgm_file(input_path: &Path, out_data: &mut Vec<u8>, assume_vgz: bool) -> Result<usize, std::io::Error> {
    let prev_size = out_data.len();

    let is_vgz = if assume_vgz {
        Ok(true)
    } else {
        detect_compression(input_path)
    };
   
    if !(is_vgz?) {
        let mut file = File::open(input_path)?;
        file.read_to_end(out_data)?;
    } else {
        print!("Deflating..");
        let mut gz_data = Vec::new();
        let mut file = File::open(input_path)?;
        file.read_to_end(&mut gz_data)?;
        let mut gz_slice = gz_data.as_slice();
        let mut gz_decoder = GzDecoder::new(&mut gz_slice);
        gz_decoder.read_to_end(out_data)?;
        println!(" done ({} -> {} bytes).", gz_data.len(), out_data.len() - prev_size);
    }
    
    Ok(out_data.len() - prev_size)
}

fn detect_compression(input_path: &Path) -> Result<bool, std::io::Error> {
    let is_vgz = if input_path.to_str().unwrap().to_lowercase().ends_with(".vgz") {
        true
    } else {  
        // Try to auto-detect .vgz files that have been named .vgm based on the first four bytes
        let mut file = File::open(input_path)?;
        let mut magic = vec![0u8; 4];
        file.read_exact(&mut magic)?;
        match String::from_utf8(magic) {
            Ok(s) => s != specification::VGM_MAGIC,
            Err(e) => true,	
        }
    };
    Ok(is_vgz)
}