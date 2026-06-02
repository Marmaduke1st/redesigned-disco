use std::env;
use std::path::PathBuf;

use gg_holdem_splitter::split_inputs;

fn main() {
    let mut args = env::args_os().skip(1);
    let input_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root/PokerCoreData/GG Poker/Holdem"));

    match split_inputs(&input_path) {
        Ok(result) => {
            println!(
                "processed {} source files and wrote {} hand files to {}",
                result.source_files,
                result.hand_files,
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Hands").display()
            );
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}