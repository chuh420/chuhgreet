use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Wayland,
    Tty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub exec: Vec<String>,
    pub desktop_names: Option<String>,
    pub kind: Kind,
}

impl Session {
    pub fn shell() -> Self {
        Self {
            id: "shell".into(),
            name: "shell".into(),
            exec: Vec::new(),
            desktop_names: None,
            kind: Kind::Tty,
        }
    }

    pub fn command(&self, user: &str) -> Vec<String> {
        match self.kind {
            Kind::Wayland => self.exec.clone(),
            Kind::Tty => {
                let passwd = fs::read_to_string("/etc/passwd").unwrap_or_default();
                vec![login_shell(&passwd, user), "-l".into()]
            }
        }
    }

    pub fn env(&self) -> Vec<String> {
        let mut env = Vec::new();
        match self.kind {
            Kind::Wayland => env.push("XDG_SESSION_TYPE=wayland".to_string()),
            Kind::Tty => env.push("XDG_SESSION_TYPE=tty".to_string()),
        }
        if let Some(names) = &self.desktop_names {
            env.push(format!("XDG_CURRENT_DESKTOP={names}"));
        }
        env.push(format!("XDG_SESSION_DESKTOP={}", self.id));
        env
    }
}

pub fn load(dirs: &[PathBuf], hide: &[String]) -> Vec<Session> {
    let mut sessions: Vec<Session> = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "desktop"))
            .collect();
        paths.sort();
        for path in paths {
            let Some(session) = read(&path) else {
                continue;
            };
            if hide.iter().any(|h| h == &session.id) || sessions.iter().any(|s| s.id == session.id)
            {
                continue;
            }
            sessions.push(session);
        }
    }
    sessions.push(Session::shell());
    sessions
}

fn read(path: &Path) -> Option<Session> {
    let id = path.file_stem()?.to_string_lossy().into_owned();
    parse(&id, &fs::read_to_string(path).ok()?)
}

pub fn parse(id: &str, text: &str) -> Option<Session> {
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut desktop_names = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "DesktopNames" => desktop_names = Some(value.trim_end_matches(';').replace(';', ":")),
            "Hidden" | "NoDisplay" if value == "true" => return None,
            _ => {}
        }
    }
    let exec = split(&exec?);
    if exec.is_empty() {
        return None;
    }
    Some(Session {
        id: id.to_string(),
        name: name.unwrap_or_else(|| id.to_string()),
        exec,
        desktop_names,
        kind: Kind::Wayland,
    })
}

pub fn login_shell(passwd: &str, user: &str) -> String {
    passwd
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            (fields.len() >= 7 && fields[0] == user).then(|| fields[6].trim().to_string())
        })
        .find(|shell| shell.starts_with('/') && !shell.contains(char::is_whitespace))
        .unwrap_or_else(|| "/bin/sh".into())
}

pub fn split(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args.into_iter()
        .filter(|a| !(a.len() == 2 && a.starts_with('%')))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entry() {
        let text = "[Desktop Entry]\nName=Sway\nExec=/usr/local/bin/session-sway %U\nDesktopNames=sway;wlroots;\n[Desktop Action x]\nName=Other\n";
        let s = parse("sway", text).unwrap();
        assert_eq!(s.name, "Sway");
        assert_eq!(s.exec, vec!["/usr/local/bin/session-sway"]);
        assert_eq!(s.desktop_names.as_deref(), Some("sway:wlroots"));
    }

    #[test]
    fn skips_hidden() {
        assert!(parse("x", "[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n").is_none());
        assert!(parse("x", "[Desktop Entry]\nName=X\n").is_none());
    }

    #[test]
    fn splits_quotes() {
        assert_eq!(
            split(r#"start "a b" c\d "e\"f""#),
            vec!["start", "a b", r"c\d", "e\"f"]
        );
    }

    #[test]
    fn finds_login_shell() {
        let passwd = "root:x:0:0::/root:/usr/bin/bash\nchuh:x:1000:1000::/home/chuh:/usr/bin/zsh\n";
        assert_eq!(login_shell(passwd, "chuh"), "/usr/bin/zsh");
        assert_eq!(login_shell(passwd, "nobody"), "/bin/sh");
        assert_eq!(login_shell("x:x:1:1::/:", "x"), "/bin/sh");
        assert_eq!(
            Session::shell().command("chuh").last().map(String::as_str),
            Some("-l")
        );
    }

    #[test]
    fn env_contains_desktop() {
        let s = parse(
            "sway",
            "[Desktop Entry]\nName=Sway\nExec=sway\nDesktopNames=sway\n",
        )
        .unwrap();
        assert!(s.env().contains(&"XDG_CURRENT_DESKTOP=sway".to_string()));
        assert!(s.env().contains(&"XDG_SESSION_TYPE=wayland".to_string()));
    }
}
