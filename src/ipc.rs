use std::env;
use std::error::Error;
use std::os::unix::net::UnixStream;

use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{AuthMessageType, Request, Response};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
pub enum Step {
    Prompt { text: String, secret: bool },
    Ready,
    Failed(String),
}

pub struct Greetd {
    stream: UnixStream,
    pub notes: Vec<String>,
}

impl Greetd {
    pub fn connect() -> Result<Self> {
        let path = env::var("GREETD_SOCK").map_err(|_| "GREETD_SOCK is not set")?;
        Ok(Self {
            stream: UnixStream::connect(path)?,
            notes: Vec::new(),
        })
    }

    fn request(&mut self, request: Request) -> Result<Response> {
        request.write_to(&mut self.stream)?;
        Ok(Response::read_from(&mut self.stream)?)
    }

    pub fn begin(&mut self, username: &str) -> Result<Step> {
        self.notes.clear();
        let response = self.request(Request::CreateSession {
            username: username.to_string(),
        })?;
        self.interpret(response)
    }

    pub fn answer(&mut self, response: Option<String>) -> Result<Step> {
        let reply = self.request(Request::PostAuthMessageResponse { response })?;
        self.interpret(reply)
    }

    pub fn start(&mut self, cmd: Vec<String>, env: Vec<String>) -> Result<Step> {
        let reply = self.request(Request::StartSession { cmd, env })?;
        self.interpret(reply)
    }

    pub fn cancel(&mut self) {
        let _ = self.request(Request::CancelSession);
    }

    fn interpret(&mut self, mut response: Response) -> Result<Step> {
        loop {
            match response {
                Response::Success => return Ok(Step::Ready),
                Response::Error { description, .. } => {
                    self.cancel();
                    return Ok(Step::Failed(description));
                }
                Response::AuthMessage {
                    auth_message_type,
                    auth_message,
                } => match auth_message_type {
                    AuthMessageType::Secret => {
                        return Ok(Step::Prompt {
                            text: auth_message,
                            secret: true,
                        });
                    }
                    AuthMessageType::Visible => {
                        return Ok(Step::Prompt {
                            text: auth_message,
                            secret: false,
                        });
                    }
                    AuthMessageType::Info | AuthMessageType::Error => {
                        let note = auth_message.trim().to_string();
                        if !note.is_empty() {
                            self.notes.push(note);
                        }
                        response =
                            self.request(Request::PostAuthMessageResponse { response: None })?;
                    }
                },
            }
        }
    }
}
