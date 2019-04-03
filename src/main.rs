
// for reading command line arguments
use std::env;

mod error;
mod file;
mod export;

use export::Exporter;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (format, in_filename, out_filename, block) = match read_cli_args(args) {
        None    => return,
        Some(s) => s,
    };



    let exporter = match Exporter::from_file(&in_filename) {
        Err(e) => {
            println!("Error: {}", e);
            return;
        },
        Ok(ex) => ex,
    };
    match exporter.export(&format, &block, &out_filename) {
        Err(e) => {
            println!("Error: {}", e);
            return;
        },
        _      => {},
    }
}

fn read_cli_args(args: Vec<String>) -> Option<(String, String, Option<String>,
                                                            Option<String>)> {
    let mut in_filename = String::new();
    let mut format      = None;
    let mut out_opt     = None;
    let mut block_opt   = None;

    let mut wait_block  = false;
    let mut wait_out    = false;

    for i in 1..args.len() {
        match args[i].as_str() {
            "-b" => wait_block = true,
            "-o" => wait_out   = true,
            _    => {
                if wait_block {
                    block_opt = Some(args[i].clone());
                    wait_block = false;
                } else if wait_out {
                    out_opt = Some(args[i].clone());
                    wait_out = false;
                } else if format.is_none() {
                    format = Some(args[i].clone());
                } else {
                    in_filename = args[i].clone();
                }
            },
        }
    }

    if args.len() < 3 || format.is_none() {
        print_help();
        return None;
    }

    Some((format.unwrap(), in_filename, out_opt, block_opt))
}

fn print_help() {
    let msg = r#"
usage:  exorg <format> <file> [-b <block name>] [-o <output file>]
        exorg [--help]
    
arguments:

    <format>        output format, valid choices:
                        - pdf           (requires installed emacs and pdflatex)
                        - pdf-minted    (much nicer-looking source code)
                        - jupyter
                        - .             extract all src blocks with a '#+FILE:'
                                        header parameter to the given paths.
                        - custom format, defined in .org file via
                                '#+SRC_LANG: <language name> <file suffix>'
                           e.g. '#+SRC_LANG: rust rs'

    <block name>    name of a specific code block to be extracted. If this block
                    depends on other blocks, those will be included as well.
                    (set via '#+NAME: <name>' before src block)

    <output file>   name of the exported src file. Default is name of .org input
                    file with the suffix replaced. This argument disables
                    automatic file suffix.
    "#; 
    println!("{}", msg);
}
