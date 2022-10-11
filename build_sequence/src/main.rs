use std::path::Path;
use build_sequence::build_directory::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    dir_to_build:String
}

fn main() {
    let args = Args::parse();
    let wd = Path::new(&args.dir_to_build);
    build_directory(&wd);
}
