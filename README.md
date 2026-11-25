# chuhgreet

English · [Русский](README.ru.md)

a TUI login greeter for [greetd](https://sr.ht/~kennylevinsen/greetd/), written in Rust with [ratatui](https://ratatui.rs). it shows the clock, the hostname, a user field, a session picker and a password field, and starts the session you pick.

## features

- reads sessions from `wayland-sessions` desktop files, so any compositor with one works: Hyprland, sway, niri and others;
- a built-in `shell` session that runs the user's login shell from `/etc/passwd`, for when a compositor will not start;
- remembers the last user and session;
- hides desktop entries you list in the config, for example the stock ones when you use your own wrappers;
- shows PAM messages and handles multi-step prompts from greetd, not only a single password;
- reboot and poweroff through `systemctl` on `F2` and `F3`;
- a preview mode with fake auth to check the UI from any terminal.

## keys

| key | action |
| --- | --- |
| `Tab`, `↓` / `Shift+Tab`, `↑` | next / previous field |
| `←`, `→` on the session field | pick a session |
| `Enter` | log in |
| `Esc` | clear the password and the error |
| `Ctrl+U` | clear the field |
| `Ctrl+L` | redraw the screen |
| `F2` | reboot |
| `F3` | poweroff |
| `Ctrl+C` | quit (preview only) |

## build

needs Rust 1.85 or newer (edition 2024).

```sh
cargo build --release
sudo install -m755 target/release/chuhgreet /usr/local/bin/chuhgreet
```

try it from a terminal first. the password is `preview`:

```sh
chuhgreet --preview
```

## install

1. point greetd at chuhgreet in `/etc/greetd/config.toml`:

   ```toml
   [terminal]
   vt = 1

   [default_session]
   command = "/usr/local/bin/chuhgreet"
   user = "greeter"
   ```

2. give the `greeter` user a place for its state file:

   ```sh
   sudo install -d -o greeter -g greeter /var/cache/chuhgreet
   ```

3. optionally write `/etc/chuhgreet.toml`. every key has a default:

   ```toml
   default_user = "me"
   hide = ["hyprland", "hyprland-uwsm", "sway"]
   session_dirs = ["/usr/local/share/wayland-sessions", "/usr/share/wayland-sessions"]
   state = "/var/cache/chuhgreet/state.toml"
   ```

   `hide` takes desktop file names without `.desktop`. when two folders hold a file with the same name, the first folder wins.

4. enable greetd: `sudo systemctl enable greetd.service`.

`dist/` holds the files from my machine: the greetd and chuhgreet configs and two session wrappers for Hyprland and sway. the wrappers point at scripts in my home folder, so change the `Exec` lines before you copy them.

## notes

- greetd runs the session command through `/bin/sh` and splits it on whitespace, so a session `Exec` with spaces inside one argument will break. put such commands in a script.
- chuhgreet sets `XDG_SESSION_TYPE`, `XDG_SESSION_DESKTOP` and, from `DesktopNames`, `XDG_CURRENT_DESKTOP`.
- only Wayland sessions are supported. X11 `xsessions` are not read.

## license

[MIT](LICENSE)
