/// vgm2spc
/// Mic, 2010,2019

#[macro_use]
extern crate bitflags;
extern crate flate2;

use std::env;
use std::path::Path;
use std::process;
use converter::*;
mod bytestream;
mod codec;
mod converter;
mod vgm;

fn show_help() {
    println!("Usage: vgm2spc <input> <output>");
    process::exit(0);
}

fn main() {
    println!("VGM to SPC Converter by Mic, 2019");

    let mut flags = converter::ConverterFlags::empty();
    let mut input_path = String::from("");
    let mut output_path = String::from("");
    
    let args: Vec<String> = env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            // Ignore args[0] (the executable)
        } else if arg.starts_with("-") {
            if arg == "-h" || arg == "-help" || arg == "-?" {
                show_help()
            } else if arg == "-raw" {
                flags |= converter::ConverterFlags::RAW_OUTPUT;
            } else {
                panic!("Unknown option: {}", arg);
            }
        } else if input_path.is_empty() {
            input_path = arg.to_owned();
        } else if output_path.is_empty() {
            output_path = arg.to_owned();
        } else {
            panic!("Unknown option: {}", arg);
        }
    }

    if input_path.is_empty() || output_path.is_empty() {
        show_help();
    }    
 
    flags |= ConverterFlags::PSG_CODEC;

    let mut converter = converter::Converter::new();
    converter.convert(Path::new(&input_path), Path::new(&output_path), flags).expect("Failed");
    println!("Done");
}
