use sonus_core::types::SyncedLine;

fn strip_inline_timestamps(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut temp = String::new();
            let mut found_end = false;
            while let Some(&next_c) = chars.peek() {
                if next_c == '>' {
                    chars.next();
                    found_end = true;
                    break;
                }
                temp.push(chars.next().unwrap());
            }
            if found_end {
                if temp.contains(':') && temp.chars().all(|ch| ch.is_ascii_digit() || ch == ':' || ch == '.' || ch == '_') {
                    continue;
                } else {
                    result.push('<');
                    result.push_str(&temp);
                    result.push('>');
                }
            } else {
                result.push('<');
                result.push_str(&temp);
            }
        } else {
            result.push(c);
        }
    }
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_lrc(lrc_text: &str) -> Vec<SyncedLine> {
    let mut lines = Vec::new();
    for line in lrc_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut text_start = 0;
        let mut timestamps = Vec::new();
        while let Some(end_idx) = line[text_start..].find(']') {
            let start_bracket = line[text_start..].find('[');
            if let Some(start_idx) = start_bracket {
                if start_idx < end_idx {
                    let ts_str = &line[text_start + start_idx + 1..text_start + end_idx];
                    if let Some(secs) = parse_lrc_timestamp(ts_str) {
                        timestamps.push(secs);
                    }
                    text_start = text_start + end_idx + 1;
                    continue;
                }
            }
            break;
        }
        let raw_text = line[text_start..].trim();
        let text = strip_inline_timestamps(raw_text);
        for ts in timestamps {
            lines.push(SyncedLine { timestamp: ts, text: text.clone() });
        }
    }
    lines.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn parse_lrc_timestamp(ts_str: &str) -> Option<f64> {
    sonus_core::util::parse_time_string(ts_str)
}
