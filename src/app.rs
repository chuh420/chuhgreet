use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{Config, State};
use crate::ipc::{Greetd, Result, Step};
use crate::power;
use crate::sessions::Session;

pub trait Auth {
    fn begin(&mut self, username: &str) -> Result<Step>;
    fn answer(&mut self, response: Option<String>) -> Result<Step>;
    fn start(&mut self, cmd: Vec<String>, env: Vec<String>) -> Result<Step>;
    fn cancel(&mut self);
    fn notes(&self) -> Vec<String>;
}

impl Auth for Greetd {
    fn begin(&mut self, username: &str) -> Result<Step> {
        Greetd::begin(self, username)
    }
    fn answer(&mut self, response: Option<String>) -> Result<Step> {
        Greetd::answer(self, response)
    }
    fn start(&mut self, cmd: Vec<String>, env: Vec<String>) -> Result<Step> {
        Greetd::start(self, cmd, env)
    }
    fn cancel(&mut self) {
        Greetd::cancel(self)
    }
    fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }
}

pub struct Preview;

impl Auth for Preview {
    fn begin(&mut self, _: &str) -> Result<Step> {
        Ok(Step::Prompt {
            text: "Password: ".into(),
            secret: true,
        })
    }
    fn answer(&mut self, response: Option<String>) -> Result<Step> {
        Ok(match response.as_deref() {
            Some("preview") => Step::Ready,
            _ => Step::Failed("authentication failed".into()),
        })
    }
    fn start(&mut self, _: Vec<String>, _: Vec<String>) -> Result<Step> {
        Ok(Step::Ready)
    }
    fn cancel(&mut self) {}
    fn notes(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    User,
    Session,
    Secret,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    Info(String),
    Error(String),
}

pub struct App {
    pub host: String,
    pub user: String,
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub secret: String,
    pub focus: Focus,
    pub status: Status,
    pub prompt: Option<(String, bool)>,
    pub busy: bool,
    pub preview: bool,
    pub quit: bool,
    state_path: PathBuf,
    auth: Box<dyn Auth>,
}

impl App {
    pub fn new(config: Config, sessions: Vec<Session>, auth: Box<dyn Auth>, preview: bool) -> Self {
        let state = State::load(&config.state);
        let user = state
            .user
            .filter(|u| !u.is_empty())
            .unwrap_or(config.default_user);
        let selected = state
            .session
            .and_then(|id| sessions.iter().position(|s| s.id == id))
            .unwrap_or(0);
        let focus = if user.is_empty() {
            Focus::User
        } else {
            Focus::Secret
        };
        Self {
            host: hostname(),
            user,
            sessions,
            selected,
            secret: String::new(),
            focus,
            status: Status::Idle,
            prompt: None,
            busy: false,
            preview,
            quit: false,
            state_path: config.state,
            auth,
        }
    }

    pub fn session(&self) -> &Session {
        &self.sessions[self.selected]
    }

    pub fn in_auth(&self) -> bool {
        self.prompt.is_some()
    }

    pub fn key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::F(2) => self.power("reboot"),
            KeyCode::F(3) => self.power("poweroff"),
            KeyCode::Char('c') if ctrl && self.preview => self.quit = true,
            KeyCode::Char('u') if ctrl => {
                if let Some(f) = self.field() {
                    f.clear();
                }
            }
            KeyCode::Esc => self.reset(None),
            KeyCode::Tab | KeyCode::Down if !self.in_auth() => self.move_focus(1),
            KeyCode::BackTab | KeyCode::Up if !self.in_auth() => self.move_focus(-1),
            KeyCode::Left if self.focus == Focus::Session && !self.in_auth() => self.cycle(-1),
            KeyCode::Right if self.focus == Focus::Session && !self.in_auth() => self.cycle(1),
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                if let Some(f) = self.field() {
                    f.pop();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(f) = self.field() {
                    f.push(c);
                }
            }
            _ => {}
        }
    }

    fn field(&mut self) -> Option<&mut String> {
        if self.in_auth() {
            return Some(&mut self.secret);
        }
        match self.focus {
            Focus::User => Some(&mut self.user),
            Focus::Secret => Some(&mut self.secret),
            Focus::Session => None,
        }
    }

    fn move_focus(&mut self, delta: i32) {
        let order = [Focus::User, Focus::Session, Focus::Secret];
        let i = order.iter().position(|f| *f == self.focus).unwrap_or(0) as i32;
        self.focus = order[(i + delta).rem_euclid(order.len() as i32) as usize];
    }

    fn cycle(&mut self, delta: i32) {
        let n = self.sessions.len() as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
    }

    fn power(&mut self, action: &str) {
        if self.preview {
            self.status = Status::Info(format!("{action} (preview)"));
            return;
        }
        if let Err(e) = power::run(action) {
            self.status = Status::Error(e);
        }
    }

    fn reset(&mut self, error: Option<String>) {
        if self.in_auth() {
            self.auth.cancel();
        }
        self.prompt = None;
        self.secret.clear();
        self.status = error.map(Status::Error).unwrap_or(Status::Idle);
    }

    fn submit(&mut self) {
        if !self.in_auth() {
            match self.focus {
                Focus::User => {
                    self.focus = Focus::Session;
                    return;
                }
                Focus::Session => {
                    self.focus = Focus::Secret;
                    return;
                }
                Focus::Secret => {}
            }
            if self.user.trim().is_empty() {
                self.focus = Focus::User;
                self.status = Status::Error("enter a user name".into());
                return;
            }
        }
        let result = if self.in_auth() {
            let answer = std::mem::take(&mut self.secret);
            self.auth.answer(Some(answer))
        } else {
            let password = std::mem::take(&mut self.secret);
            let user = self.user.trim().to_string();
            self.auth.begin(&user).and_then(|step| match step {
                Step::Prompt { .. } => self.auth.answer(Some(password)),
                other => Ok(other),
            })
        };
        self.handle(result);
    }

    fn handle(&mut self, result: Result<Step>) {
        let notes = self.auth.notes();
        match result {
            Err(e) => self.reset(Some(e.to_string())),
            Ok(Step::Failed(e)) => {
                self.prompt = None;
                self.secret.clear();
                let message = notes.last().cloned().unwrap_or(e);
                self.status = Status::Error(lower(&message));
            }
            Ok(Step::Prompt { text, secret }) => {
                self.prompt = Some((lower(text.trim().trim_end_matches(':')), secret));
                self.status = notes
                    .last()
                    .map(|n| Status::Info(lower(n)))
                    .unwrap_or(Status::Idle);
            }
            Ok(Step::Ready) => self.launch(),
        }
    }

    fn launch(&mut self) {
        let session = self.session().clone();
        State {
            user: Some(self.user.trim().to_string()),
            session: Some(session.id.clone()),
        }
        .save(&self.state_path);
        self.status = Status::Info(format!("starting {}", session.name.to_lowercase()));
        match self
            .auth
            .start(session.command(self.user.trim()), session.env())
        {
            Ok(Step::Ready) => self.quit = true,
            Ok(Step::Failed(e)) => self.reset(Some(lower(&e))),
            Ok(Step::Prompt { .. }) => self.reset(Some("unexpected prompt".into())),
            Err(e) => self.reset(Some(e.to_string())),
        }
    }
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "login".into())
}
