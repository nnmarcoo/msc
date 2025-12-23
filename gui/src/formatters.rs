pub fn format_duration(seconds: f32) -> String {
    let total_secs = seconds as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

pub fn format_optional_u32(value: Option<u32>, unit: &str) -> String {
    match value {
        Some(v) => format!("{} {}", v, unit),
        None => "-".to_string(),
    }
}

pub fn format_optional_u8(value: Option<u8>, unit: &str) -> String {
    match value {
        Some(v) => format!("{} {}", v, unit),
        None => "-".to_string(),
    }
}

pub fn format_sample_rate(sample_rate: Option<u32>) -> String {
    match sample_rate {
        Some(rate) => {
            if rate >= 1000 {
                format!("{:.1} kHz", rate as f32 / 1000.0)
            } else {
                format!("{} Hz", rate)
            }
        }
        None => "-".to_string(),
    }
}

pub fn format_channels(channels: Option<u8>) -> String {
    match channels {
        Some(1) => "Mono".to_string(),
        Some(2) => "Stereo".to_string(),
        Some(n) => format!("{} channels", n),
        None => "-".to_string(),
    }
}
