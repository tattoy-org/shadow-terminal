# Shadow Terminal
A fully-functional, fully-rendered terminal emulator in memory.

If you're aware of headless browsers then Shadow Terminal is just like that, but for terminals. Similarly, Shadow Terminal can be used for End To End testing TUI applications. But it can also be used as a basis for making terminal multiplexers (a la `tmux`, `zellij`).

Existing tools for testing CLI apps, like Python's [`pexpect`](https://github.com/pexpect/pexpect), generally only work at the PTY level, meaning they don't fully parse and handle all ANSI codes. For example a PTY doesn't maintain a grid of cells into which you can query. Shadow Terminal on the other hand can render applications like `top` and `vim` whilst providing a convenient API to get all the attribute details of each cell, like true colour values, etc.

The underlying terminal is Wezterm's [`wezterm-term`](https://github.com/wezterm/wezterm/tree/main/term), which is the core of the Wezterm GUI terminal emulator. So anything that Wezterm can do Shadow Terminal should also be able to do.

## Usage

### `shadow-terminal` CLI

Pre-built binaries for Linux, Mac OS and Windows are available in our Github [Releases](https://github.com/tattoy-org/shadow-terminal/releases).

The `shadow-terminal` CLI starts a headless terminal running an arbitrary command. It can even start interactive commands like `bash`, forwarding the user's STDIN to the underlying terminal. Though note that it doesn't automatically put your own terminal into "raw" mode, which means that all input is buffered and only forwarded when sending a newline (like when pressing the Enter key). This limitation does not exist when running `shadow-terminal` as a subprocess from some other code.

By default the underlying terminal's output is sent to STDOUT as rich JSON object. A new object is sent for every change to the underlying terminal. The structure of this JSON can be found in the [JSON schema](/output-schema.json) at the root of this repo.

You may also render the output as plain text. This most likely is only useful for quick debugging.

Roadmap:
* [ ] Support sending the resize and scrolling triggers.
* [ ] Support outputting the scrollback buffer.

Here are the full usage details:
```
Usage: shadow-terminal [OPTIONS] [COMMAND]...

Arguments:
  [COMMAND]...
          The command to run in the shadow terminal

          [env: SHELL=/usr/bin/fish]
          [default: bash]

Options:
      --width <WIDTH>
          The width of the shadow terminal

      --height <HEIGHT>
          The height of the shadow terminal

      --scrollback-size <SCROLLBACK_SIZE>
          The number of lines for the shadow terminal's scrollback buffer

          [default: 1000]

      --output <OUTPUT>
          The format to return the output of the shadow terminal

          [default: json]

          Possible values:
          - json:  A rich and structured representation of all the cells' data
          - plain: Just a plain, monochrome format useful for debugging

      --generate-schema
          Generate the current JSON schema for serialised output

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Rust crate

Shadow Terminal has 2 modes of running: `ActiveTerminal` and `SteppableTerminal`.

Full docs are available at: https://docs.rs/shadow-terminal/latest/shadow_terminal

#### `ActiveTerminal`

This is more useful for realtime applications such as terminal multiplexers for example.

```rust
let shadow_terminal = ActiveTerminal::start(Config::default());
forward_stdin(shadow_terminal.pty_input_tx.clone());
while let Some(output) = shadow_terminal.surface_output_rx.recv().await {
    // ...
    dbg!(surface);
}
```

#### `SteppableTerminal`

This is more useful for E2E testing. The underlying terminal doesn't automatically parse the output
from its PTY. This allows for conveniently stepping through the state of the run command.

```rust
let mut stepper = SteppableTerminal::start(Config::default()).await.unwrap();
stepper.send_command("echo $((1+1))").unwrap();
stepper.wait_for_any_change().await.unwrap();
let output = stepper.screen_as_string().unwrap();
assert_eq!(
    output,
    indoc::formatdoc! {"
        {prompt} echo $((1+1))
        2
        {prompt} 
    "}
);
```

### C-compatible library

Coming soon...

## Project currently using Shadow Terminal
* [Tattoy](https://tattoy.sh) uses the Shadow Terminal both for rendering compositing effects and for E2E tetsing.

### Similar Projects
* https://github.com/gdamore/tcell (Golang, more similar in scope to https://github.com/ratatui/ratatui)
* https://github.com/pexpect/pexpect (Python, with various bindings to other languages)
* https://github.com/rust-cli/rexpect
* https://github.com/doy/vt100-rust
