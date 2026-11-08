use std::process::{Command, Stdio};

pub fn run(action: &str) -> Result<(), String> {
    let status = Command::new("systemctl")
        .arg(action)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{action} failed"))
    }
}
