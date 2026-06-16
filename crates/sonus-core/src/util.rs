pub fn is_valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn parse_time_string(s: &str) -> Option<f64> {
    let mut parts = s.split(':');
    let first = parts.next()?;
    let second = parts.next();
    let third = parts.next();
    if parts.next().is_some() {
        return None;
    }
    match (first, second, third) {
        (s1, None, None) => s1.parse::<f64>().ok(),
        (s1, Some(s2), None) => {
            let mm = s1.parse::<f64>().ok()?;
            let ss = s2.parse::<f64>().ok()?;
            Some(mm * 60.0 + ss)
        }
        (s1, Some(s2), Some(s3)) => {
            let hh = s1.parse::<f64>().ok()?;
            let mm = s2.parse::<f64>().ok()?;
            let ss = s3.parse::<f64>().ok()?;
            Some(hh * 3600.0 + mm * 60.0 + ss)
        }
        _ => None,
    }
}

pub fn fit_to_width<'a>(s: &'a str, target_width: usize, truncate_suffix: &str) -> std::borrow::Cow<'a, str> {
    use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};
    let s_width = s.width();
    if s_width == target_width {
        return std::borrow::Cow::Borrowed(s);
    }
    if s_width > target_width {
        let suffix_width = truncate_suffix.width();
        let max_content_width = target_width.saturating_sub(suffix_width);

        let mut current_width = 0;
        let mut result = String::new();
        for c in s.chars() {
            let c_width = UnicodeWidthChar::width(c).unwrap_or(0);
            if current_width + c_width > max_content_width {
                break;
            }
            result.push(c);
            current_width += c_width;
        }
        let padding = max_content_width.saturating_sub(current_width);
        result.extend(std::iter::repeat(' ').take(padding));
        result.push_str(truncate_suffix);
        std::borrow::Cow::Owned(result)
    } else {
        let padding = target_width - s_width;
        let mut result = String::with_capacity(s.len() + padding);
        result.push_str(s);
        result.extend(std::iter::repeat(' ').take(padding));
        std::borrow::Cow::Owned(result)
    }
}
