// Borked org

// for reading command line arguments
use std::env;

mod error;
mod file;
mod export;

use export::Exporter;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let (format, filename, block) = match read_cli_args(args) {
        None    => return,
        Some(s) => s,
    };



    let exporter = match Exporter::from_file(&filename) {
        Err(e) => panic!("Error: {:?}", e),
        Ok(ex) => ex,
    };
    match exporter.export(&format, &block) {
        Err(e) => panic!("Error: {:?}", e),
        _      => {},
    }
}

fn read_cli_args(args: Vec<String>) -> Option<(String, String, Option<String>)> {
    match args.len() {
        3 => Some( (args[1].to_owned(), args[2].to_owned(), None) ),
        4 => Some( (args[1].to_owned(), args[2].to_owned(), Some(args[3].to_owned())) ),
        _ => {
            print_help();
            None
        },
    }
}

fn print_help() {
    // TODO document more precisely how e.g. programming language handling should be defined inside
    // source file
    let msg = r#"
usage:  borg <format> <file> [<block name>]
        borg [--help]
    
arguments:

    <format>        output format, valid choices are either "pdf"(requires emacs
                    and latex) or the name of the programming language to be
                    exported.

    <block name>    name of a specific code block to be extracted. If this block
                    depends on other blocks, those will be included as well.

    --help          print this message and exit
    "#; 
    println!("{}", msg);
}
