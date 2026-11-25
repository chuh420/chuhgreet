# chuhgreet

[English](README.md) · Русский

TUI-экран входа для [greetd](https://sr.ht/~kennylevinsen/greetd/) на Rust и [ratatui](https://ratatui.rs). на экране часы, имя компьютера, поле пользователя, выбор сессии и поле пароля. после входа chuhgreet запускает выбранную сессию.

## возможности

- берёт сессии из файлов `wayland-sessions`, поэтому подходит любой композитор, у которого такой файл есть: Hyprland, sway, niri и другие;
- встроенная сессия `shell` запускает оболочку пользователя из `/etc/passwd`, на случай если композитор не стартует;
- помнит последнего пользователя и сессию;
- прячет записи из списка в конфиге, например стандартные, если у тебя свои обёртки;
- показывает сообщения PAM и поддерживает многошаговые запросы greetd, а не только один пароль;
- перезагрузка и выключение через `systemctl` по `F2` и `F3`;
- режим просмотра с ненастоящим входом, чтобы проверить интерфейс в любом терминале.

## клавиши

| клавиша | действие |
| --- | --- |
| `Tab`, `↓` / `Shift+Tab`, `↑` | следующее / предыдущее поле |
| `←`, `→` на поле session | выбор сессии |
| `Enter` | вход |
| `Esc` | очистить пароль и ошибку |
| `Ctrl+U` | очистить поле |
| `Ctrl+L` | перерисовать экран |
| `F2` | перезагрузка |
| `F3` | выключение |
| `Ctrl+C` | выход (только в режиме просмотра) |

## сборка

нужен Rust 1.85 или новее (edition 2024).

```sh
cargo build --release
sudo install -m755 target/release/chuhgreet /usr/local/bin/chuhgreet
```

сначала попробуй в терминале. пароль `preview`:

```sh
chuhgreet --preview
```

## установка

1. укажи chuhgreet в `/etc/greetd/config.toml`:

   ```toml
   [terminal]
   vt = 1

   [default_session]
   command = "/usr/local/bin/chuhgreet"
   user = "greeter"
   ```

2. создай папку для файла состояния, владелец `greeter`:

   ```sh
   sudo install -d -o greeter -g greeter /var/cache/chuhgreet
   ```

3. при желании напиши `/etc/chuhgreet.toml`. у каждого ключа есть значение по умолчанию:

   ```toml
   default_user = "me"
   hide = ["hyprland", "hyprland-uwsm", "sway"]
   session_dirs = ["/usr/local/share/wayland-sessions", "/usr/share/wayland-sessions"]
   state = "/var/cache/chuhgreet/state.toml"
   ```

   в `hide` пиши имена файлов без `.desktop`. если файл с одним именем лежит в двух папках, chuhgreet берёт его из первой.

4. включи greetd: `sudo systemctl enable greetd.service`.

в `dist/` лежат файлы с моей машины: конфиги greetd и chuhgreet и две обёртки сессий для Hyprland и sway. обёртки ссылаются на скрипты в моей домашней папке, поэтому перед копированием поменяй строки `Exec`.

## замечания

- greetd запускает команду сессии через `/bin/sh` и делит её по пробелам. если в `Exec` есть аргумент с пробелом внутри, вход сломается. такие команды выноси в скрипт.
- chuhgreet задаёт `XDG_SESSION_TYPE`, `XDG_SESSION_DESKTOP` и, из `DesktopNames`, `XDG_CURRENT_DESKTOP`.
- поддерживаются только сессии Wayland. файлы X11 из `xsessions` chuhgreet не читает.

## лицензия

[MIT](LICENSE)
