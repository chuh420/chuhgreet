use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_user: String,
    pub session_dirs: Vec<PathBuf>,
    pub hide: Vec<String>,
    pub state: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_user: String::new(),
            session_dirs: vec![
                "/usr/local/share/wayland-sessions".into(),
                "/usr/share/wayland-sessions".into(),
            ],
            hide: Vec::new(),
            state: "/var/cache/chuhgreet/state.toml".into(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct State {
    pub user: Option<String>,
    pub session: Option<String>,
}

impl State {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(text) = toml::to_string(self) {
            let tmp = path.with_extension("tmp");
            if fs::write(&tmp, text).is_ok() {
                let _ = fs::rename(&tmp, path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_fill_missing() {
        let c: Config = toml::from_str("hide = [\"hyprland\"]").unwrap();
        assert_eq!(c.hide, vec!["hyprland"]);
        assert_eq!(c.session_dirs.len(), 2);
    }

    #[test]
    fn state_roundtrip() {
        let dir = std::env::temp_dir().join(format!("chuhgreet-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.toml");
        let state = State {
            user: Some("chuh".into()),
            session: Some("chuh-sway".into()),
        };
        state.save(&path);
        assert_eq!(State::load(&path), state);
        fs::remove_dir_all(dir).unwrap();
    }
}
