use std::error::Error;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SplitError {
    Io(std::io::Error),
    MissingHandHeader(PathBuf),
    DuplicateHandId { hand_id: String, first: PathBuf, second: PathBuf },
    InvalidOutputPath(PathBuf),
}

impl Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitError::Io(err) => write!(f, "I/O error: {err}"),
            SplitError::MissingHandHeader(path) => {
                write!(f, "no GG hand headers found in {}", path.display())
            }
            SplitError::DuplicateHandId { hand_id, first, second } => write!(
                f,
                "duplicate hand id {hand_id} found in {} and {}",
                first.display(),
                second.display()
            ),
            SplitError::InvalidOutputPath(path) => {
                write!(f, "output path cannot be inside input scan path: {}", path.display())
            }
        }
    }
}

impl Error for SplitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SplitError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SplitError {
    fn from(value: std::io::Error) -> Self {
        SplitError::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitResult {
    pub source_files: usize,
    pub hand_files: usize,
}

pub fn split_inputs(input_path: &Path) -> Result<SplitResult, SplitError> {
    let output_dir = default_output_dir();

    if !input_path.exists() {
        return Err(SplitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("input path does not exist: {}", input_path.display()),
        )));
    }

    if input_path.is_file() {
        let hand_files = split_file(input_path, &output_dir)?;
        return Ok(SplitResult {
            source_files: 1,
            hand_files,
        });
    }

    let input_root = input_path.canonicalize()?;
    let output_root = output_dir.canonicalize().or_else(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Ok(output_dir.to_path_buf())
        } else {
            Err(err)
        }
    })?;

    if output_root.starts_with(&input_root) {
        return Err(SplitError::InvalidOutputPath(output_root));
    }

    let mut source_files = 0;
    let mut hand_files = 0;

    for entry in walk_txt_files(&input_root)? {
        source_files += 1;
        hand_files += split_file(&entry, &output_dir)?;
    }

    Ok(SplitResult {
        source_files,
        hand_files,
    })
}

fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Hands")
}

pub fn split_file(source_path: &Path, output_dir: &Path) -> Result<usize, SplitError> {
    let text = fs::read_to_string(source_path)?;
    let hand_ranges = hand_ranges(&text);

    if hand_ranges.is_empty() {
        return Err(SplitError::MissingHandHeader(source_path.to_path_buf()));
    }

    fs::create_dir_all(output_dir)?;

    let mut written = 0;
    for (start, end, hand_id) in hand_ranges {
        let hand_text = text[start..end].trim_end();
        let target_path = output_dir.join(format!("{hand_id}.txt"));

        if target_path.exists() {
            let existing = fs::read_to_string(&target_path)?;
            if existing == hand_text {
                continue;
            }

            return Err(SplitError::DuplicateHandId {
                hand_id,
                first: source_path.to_path_buf(),
                second: target_path,
            });
        }

        fs::write(target_path, format!("{hand_text}\n"))?;
        written += 1;
    }

    Ok(written)
}

fn walk_txt_files(root: &Path) -> Result<Vec<PathBuf>, SplitError> {
    let mut files = Vec::new();
    collect_txt_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_txt_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), SplitError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let name = entry.file_name();
            if name.to_string_lossy() == "Hands" {
                continue;
            }
            collect_txt_files(&entry_path, out)?;
            continue;
        }

        if entry_path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            out.push(entry_path);
        }
    }

    Ok(())
}

fn hand_ranges(text: &str) -> Vec<(usize, usize, String)> {
    let mut starts: Vec<(usize, String)> = Vec::new();

    for (index, _) in text.match_indices("Poker Hand #") {
        if let Some(hand_id) = parse_hand_id(&text[index..]) {
            starts.push((index, hand_id));
        }
    }

    let mut ranges = Vec::new();
    for (i, (start, hand_id)) in starts.iter().enumerate() {
        let end = starts
            .get(i + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(text.len());
        ranges.push((*start, end, hand_id.clone()));
    }

    ranges
}

fn parse_hand_id(line_start: &str) -> Option<String> {
    let after_marker = line_start.strip_prefix("Poker Hand #")?;
    let hand_id = after_marker.split_once(':')?.0.trim();
    if hand_id.is_empty() {
        None
    } else {
        Some(hand_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hand_ids_from_export_text() {
        let text = concat!(
            "Poker Hand #HD1: Hold'em No Limit ($0.01/$0.02) - 2026/04/20 00:10:56\n",
            "Table 'NLHWhite111' 6-max Seat #5 is the button\n",
            "Poker Hand #HD2: Hold'em No Limit ($0.01/$0.02) - 2026/04/20 00:11:01\n",
        );

        let ranges = hand_ranges(text);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].2, "HD1");
        assert_eq!(ranges[1].2, "HD2");
    }
}