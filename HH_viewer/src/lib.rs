use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppState {
    pub hands_root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct HandFile {
    pub hand_id: String,
    pub path: PathBuf,
}

pub fn default_hands_root() -> PathBuf {
    std::env::var_os("HANDS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Hands"))
}

pub fn list_hands(root: &Path, query: Option<&str>) -> io::Result<Vec<HandFile>> {
    let mut hands = Vec::new();

    if !root.exists() {
        return Ok(hands);
    }

    if root.is_file() {
        if let Some(hand_id) = root.file_stem().and_then(|stem| stem.to_str()) {
            let query_matches = query.map(|query| {
                let query = query.trim();
                query.is_empty() || hand_id.to_lowercase().contains(&query.to_lowercase())
            }).unwrap_or(true);

            if query_matches {
                hands.push(HandFile {
                    hand_id: hand_id.to_string(),
                    path: root.to_path_buf(),
                });
            }
        }

        return Ok(hands);
    }

    collect_hands(root, query, &mut hands)?;

    hands.sort_by(|left, right| left.hand_id.cmp(&right.hand_id));
    Ok(hands)
}

pub fn find_hand_path(root: &Path, hand_id: &str) -> PathBuf {
    if root.is_file() {
        if root.file_stem().and_then(|stem| stem.to_str()) == Some(hand_id) {
            return root.to_path_buf();
        }
    }

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = find_hand_path(&path, hand_id);
                if nested.exists() {
                    return nested;
                }
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) == Some("txt")
                && path.file_stem().and_then(|stem| stem.to_str()) == Some(hand_id)
            {
                return path;
            }
        }
    }

    root.join(format!("{hand_id}.txt"))
}

pub fn read_hand_text(root: &Path, hand_id: &str) -> io::Result<String> {
    fs::read_to_string(find_hand_path(root, hand_id))
}

pub fn render_index(root: &Path, query: Option<&str>) -> io::Result<String> {
    let hands = list_hands(root, query)?;
    let mut html = String::new();

    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Hand Viewer</title>");
    html.push_str("<style>body{font-family:system-ui,sans-serif;margin:0;background:#101418;color:#e6edf3}header{padding:24px;background:#151b22;border-bottom:1px solid #263241}main{padding:24px;max-width:1100px;margin:0 auto}form{display:flex;gap:12px;flex-wrap:wrap;margin:16px 0 24px}input{flex:1;min-width:240px;padding:12px 14px;border-radius:10px;border:1px solid #334155;background:#0b1117;color:#e6edf3}button,a{padding:12px 14px;border-radius:10px;border:1px solid #334155;background:#1f2937;color:#e6edf3;text-decoration:none}table{width:100%;border-collapse:collapse;background:#0b1117;border:1px solid #243041;border-radius:12px;overflow:hidden}th,td{padding:12px 14px;border-bottom:1px solid #1f2937;text-align:left}tr:hover{background:#111827}.muted{color:#94a3b8}</style>");
    html.push_str("</head><body><header><h1>Hand Viewer</h1><div class=\"muted\">Browse, search, and download hand files from any directory or single file.</div></header><main>");

    let query_value = query.unwrap_or_default();
    let _ = write!(
        html,
        "<form method=\"get\"><input name=\"q\" value=\"{}\" placeholder=\"Search hand id\"><button type=\"submit\">Search</button><a href=\"/\">Reset</a></form>",
        escape_html(query_value)
    );

    let _ = write!(html, "<div class=\"muted\">{} hand files found</div>", hands.len());
    html.push_str("<table><thead><tr><th>Hand ID</th><th>Actions</th></tr></thead><tbody>");

    for hand in hands {
        let _ = write!(
            html,
            "<tr><td>{}</td><td><a href=\"/hand/{}\">View</a> <a href=\"/download/{}\">Download</a></td></tr>",
            escape_html(&hand.hand_id),
            url_encode(&hand.hand_id),
            url_encode(&hand.hand_id)
        );
    }

    html.push_str("</tbody></table></main></body></html>");
    Ok(html)
}

pub fn render_hand_page(hand_id: &str, hand_text: &str) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Hand ");
    html.push_str(&escape_html(hand_id));
    html.push_str("</title><style>body{font-family:system-ui,sans-serif;margin:0;background:#101418;color:#e6edf3}header{padding:24px;background:#151b22;border-bottom:1px solid #263241}main{padding:24px;max-width:1200px;margin:0 auto}.card{background:#0b1117;border:1px solid #243041;border-radius:12px;padding:16px;white-space:pre-wrap;overflow:auto}a{color:#93c5fd;text-decoration:none}.muted{color:#94a3b8}</style></head><body>");
    html.push_str("<header><h1>Hand ");
    html.push_str(&escape_html(hand_id));
    html.push_str("</h1><div class=\"muted\"><a href=\"/\">Back to list</a> | <a href=\"/download/");
    html.push_str(&url_encode(hand_id));
    html.push_str("\">Download raw file</a></div></header><main><div class=\"card\"><pre>");
    html.push_str(&escape_html(hand_text));
    html.push_str("</pre></div></main></body></html>");
    html
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn url_encode(input: &str) -> String {
    input
        .chars()
        .map(|character| match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => character.to_string(),
            _ => format!("%{:02X}", character as u32),
        })
        .collect()
}

fn collect_hands(root: &Path, query: Option<&str>, hands: &mut Vec<HandFile>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_hands(&path, query, hands)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            continue;
        }

        let Some(hand_id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };

        if let Some(query) = query {
            let query = query.trim();
            if !query.is_empty() && !hand_id.to_lowercase().contains(&query.to_lowercase()) {
                continue;
            }
        }

        hands.push(HandFile {
            hand_id: hand_id.to_string(),
            path,
        });
    }

    Ok(())
}
