mod app;
mod config;
mod ipc;
mod power;
mod sessions;
mod ui;

use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use app::{App, Auth, Preview};
use config::Config;
use ipc::Greetd;

const CONFIG: &str = "/etc/chuhgreet.toml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let preview = std::env::args().any(|a| a == "--preview");
    let mut config = Config::load(Path::new(CONFIG));
    if preview {
        config.state = std::env::temp_dir().join("chuhgreet-preview.toml");
    }
    let sessions = sessions::load(&config.session_dirs, &config.hide);
    let auth: Box<dyn Auth> = if preview {
        Box::new(Preview)
    } else {
        Box::new(Greetd::connect()?)
    };
    let mut app = App::new(config, sessions, auth, preview);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if !event::poll(Duration::from_secs(1))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if key.code == KeyCode::Enter {
                app.busy = true;
                terminal.draw(|f| ui::draw(f, app))?;
            }
            app.key(key);
            app.busy = false;
        }
    }
    io::Write::flush(&mut io::stdout())?;
    Ok(())
}
