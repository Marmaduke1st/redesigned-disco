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

#[derive(Clone, Debug)]
pub struct DashboardStats {
    pub total_hands: usize,
    pub total_bytes: u64,
    pub recent_hands: Vec<HandFile>,
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

pub fn dashboard_stats(root: &Path) -> io::Result<DashboardStats> {
    let mut total_bytes = 0_u64;
    let mut hands = Vec::new();

    if !root.exists() {
        return Ok(DashboardStats {
            total_hands: 0,
            total_bytes: 0,
            recent_hands: hands,
        });
    }

    collect_hands_for_dashboard(root, &mut hands, &mut total_bytes)?;
    hands.sort_by(|left, right| {
        let left_modified = fs::metadata(&left.path).and_then(|metadata| metadata.modified()).ok();
        let right_modified = fs::metadata(&right.path).and_then(|metadata| metadata.modified()).ok();
        right_modified
            .cmp(&left_modified)
            .then_with(|| right.hand_id.cmp(&left.hand_id))
    });

    let total_hands = hands.len();
    hands.truncate(10);

    Ok(DashboardStats {
        total_hands,
        total_bytes,
        recent_hands: hands,
    })
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

pub fn render_dashboard(root: &Path) -> io::Result<String> {
    let stats = dashboard_stats(root)?;
    let mut html = String::new();

    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Hand Dashboard</title>");
    html.push_str("<style>body{font-family:system-ui,sans-serif;margin:0;background:#0f141a;color:#e6edf3}header{padding:28px 24px;background:linear-gradient(135deg,#121a24,#0b1117);border-bottom:1px solid #233044}main{padding:24px;max-width:1200px;margin:0 auto;display:grid;gap:20px}.topbar{display:flex;justify-content:space-between;align-items:end;gap:16px;flex-wrap:wrap}.eyebrow{color:#93c5fd;text-transform:uppercase;letter-spacing:.08em;font-size:.78rem}.subtitle{color:#94a3b8;max-width:64ch}.actions{display:flex;gap:12px;flex-wrap:wrap}.btn{display:inline-flex;align-items:center;gap:8px;padding:12px 16px;border-radius:12px;border:1px solid #2f3d50;background:#17212b;color:#e6edf3;text-decoration:none}.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:16px}.card{background:#111823;border:1px solid #243041;border-radius:16px;padding:18px}.value{font-size:2rem;font-weight:700;margin-top:8px}.muted{color:#94a3b8}.recent{background:#111823;border:1px solid #243041;border-radius:16px;overflow:hidden}table{width:100%;border-collapse:collapse}th,td{padding:14px 16px;border-bottom:1px solid #1f2937;text-align:left}th{font-size:.8rem;text-transform:uppercase;letter-spacing:.06em;color:#94a3b8;background:#0c1219}tr:hover{background:#151e29}a{color:#93c5fd;text-decoration:none}</style>");
    html.push_str("</head><body><header><div class=\"topbar\"><div><div class=\"eyebrow\">Poker Core</div><h1 style=\"margin:8px 0 0\">Hand Dashboard</h1><div class=\"subtitle\">A homepage overview for your imported hand library, with the full hand browser available on a separate page.</div></div><div class=\"actions\"><a class=\"btn\" href=\"/hands\">Open Hand Viewer</a></div></div></header><main>");

    html.push_str("<section class=\"cards\">");
    let _ = write!(html, "<div class=\"card\"><div class=\"muted\">Total Hands</div><div class=\"value\">{}</div></div>", stats.total_hands);
    let _ = write!(html, "<div class=\"card\"><div class=\"muted\">Library Size</div><div class=\"value\">{}</div><div class=\"muted\">bytes</div></div>", stats.total_bytes);
    let _ = write!(html, "<div class=\"card\"><div class=\"muted\">Viewer Status</div><div class=\"value\">Ready</div><div class=\"muted\">Search, open, or download hands</div></div>");
    html.push_str("</section>");

    html.push_str("<section class=\"recent\"><table><thead><tr><th>Recent Hands</th><th>Actions</th></tr></thead><tbody>");
    if stats.recent_hands.is_empty() {
        html.push_str("<tr><td colspan=\"2\" class=\"muted\">No imported hands found.</td></tr>");
    } else {
        for hand in stats.recent_hands {
            let _ = write!(html, "<tr><td>{}</td><td><a href=\"/hand/{}\">View</a> <a href=\"/download/{}\">Download</a></td></tr>", escape_html(&hand.hand_id), url_encode(&hand.hand_id), url_encode(&hand.hand_id));
        }
    }
    html.push_str("</tbody></table></section></main></body></html>");
    Ok(html)
}

pub fn render_index(root: &Path, query: Option<&str>) -> io::Result<String> {
    let hands = list_hands(root, query)?;
    let mut html = String::new();

    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Hand Viewer</title>");
    html.push_str("<style>body{font-family:system-ui,sans-serif;margin:0;background:#0f141a;color:#e6edf3}header{padding:28px 24px;background:linear-gradient(135deg,#121a24,#0b1117);border-bottom:1px solid #233044}main{padding:24px;max-width:1200px;margin:0 auto}form{display:flex;gap:12px;flex-wrap:wrap;margin:16px 0 24px}input{flex:1;min-width:240px;padding:12px 14px;border-radius:10px;border:1px solid #334155;background:#0b1117;color:#e6edf3}button,a{padding:12px 14px;border-radius:10px;border:1px solid #334155;background:#1f2937;color:#e6edf3;text-decoration:none}table{width:100%;border-collapse:collapse;background:#111823;border:1px solid #243041;border-radius:12px;overflow:hidden}th,td{padding:12px 14px;border-bottom:1px solid #1f2937;text-align:left}tr:hover{background:#151e29}.muted{color:#94a3b8}.toplinks{display:flex;gap:12px;flex-wrap:wrap;margin-top:16px}.pill{display:inline-flex;align-items:center;padding:10px 14px;border-radius:999px;border:1px solid #334155;background:#111827;color:#e6edf3;text-decoration:none}</style>");
    html.push_str("</head><body><header><div class=\"muted\">Hand Viewer</div><h1 style=\"margin:8px 0 0\">All Hand Files</h1><div class=\"muted\">Browse, search, and download hand files from any directory or single file. The dashboard lives on the homepage.</div><div class=\"toplinks\"><a class=\"pill\" href=\"/\">Dashboard</a></div></header><main>");

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

fn collect_hands_for_dashboard(
    root: &Path,
    hands: &mut Vec<HandFile>,
    total_bytes: &mut u64,
) -> io::Result<()> {
    if root.is_file() {
        let metadata = fs::metadata(root)?;
        *total_bytes += metadata.len();
        if let Some(hand_id) = root.file_stem().and_then(|stem| stem.to_str()) {
            hands.push(HandFile {
                hand_id: hand_id.to_string(),
                path: root.to_path_buf(),
            });
        }
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_hands_for_dashboard(&path, hands, total_bytes)?;
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            continue;
        }

        *total_bytes += fs::metadata(&path)?.len();

        let Some(hand_id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };

        hands.push(HandFile {
            hand_id: hand_id.to_string(),
            path,
        });
    }

    Ok(())
}
