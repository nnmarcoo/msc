pub fn format_seconds(seconds: f32) -> String {
  let minutes = (seconds / 60.) as u32;
  let seconds = (seconds % 60.) as u32;

  format!("{:02}:{:02}", minutes, seconds)
}