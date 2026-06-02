use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gg_holdem_splitter::split_inputs;

fn main() {
    let watch_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root/PokerCoreData/GG Poker/Holdem"));

    if !watch_path.exists() {
        eprintln!("Watch path does not exist: {}", watch_path.display());
        std::process::exit(1);
    }

    println!("Watching directory: {}", watch_path.display());
    println!("Checking every 60 seconds for new hand files");
    println!("New hands will be automatically processed and split to HH_splitter/GG_holdem/Hands");

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("HH_splitter/GG_holdem/Hands");

    let mut known_files = scan_directory(&watch_path);
    println!("Initial scan found {} .txt files", known_files.len());

    loop {
        std::thread::sleep(Duration::from_secs(60));

        let current_files = scan_directory(&watch_path);
        let new_files: HashSet<_> = current_files.difference(&known_files).cloned().collect();

        if !new_files.is_empty() {
            println!("Detected {} new .txt file(s)", new_files.len());
            for file in &new_files {
                println!("  - {}", file.display());
            }

            match split_inputs(&watch_path) {
                Ok(result) => {
                    println!(
                        "Processed {} source files, wrote {} hand files to {}",
                        result.source_files,
                        result.hand_files,
                        output_dir.display()
                    );
                }
                Err(err) => {
                    eprintln!("Failed to split hands: {}", err);
                }
            }

            known_files = current_files;
        }
    }
}

fn scan_directory(root: &Path) -> HashSet<PathBuf> {
    let mut files = HashSet::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(scan_directory(&path));
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
                files.insert(path);
            }
        }
    }

    files
}
