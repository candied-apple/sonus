pub fn is_valid_video_id(id: &str) -> bool {
    sonus_core::util::is_valid_video_id(id)
}

pub fn parse_time_string(s: &str) -> Option<f64> {
    sonus_core::util::parse_time_string(s)
}

pub fn fit_to_width(s: &str, target_width: usize, truncate_suffix: &str) -> String {
    sonus_core::util::fit_to_width(s, target_width, truncate_suffix).into_owned()
}
