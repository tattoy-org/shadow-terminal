//! The application code.

use std::io::Read as _;

use clap::Parser as _;
use color_eyre::{eyre::ContextCompat as _, Result};
use shadow_terminal::termwiz::terminal::Terminal as _;

/// The main app.
pub struct App {
    /// CLI arguments.
    cli: crate::cli_args::CliArgs,
    /// A structured representation of the terminal's current screen.
    screen: shadow_terminal::output::native::CompleteScreen,
    /// The running shadow terminal itself.
    shadow_terminal: shadow_terminal::active_terminal::ActiveTerminal,
}

impl App {
    /// Run the app.
    pub async fn run() -> Result<()> {
        let cli = crate::cli_args::CliArgs::parse();

        #[expect(clippy::print_stdout, reason = "This isn't just debugging")]
        if cli.generate_schema {
            let schema = schemars::schema_for!(shadow_terminal::output::foreign::Output);
            println!("{}", serde_json::to_string_pretty(&schema)?);
            return Ok(());
        }

        let config = Self::setup(cli.clone())?;
        let screen = shadow_terminal::output::native::CompleteScreen::new(
            config.width.into(),
            config.height.into(),
        );
        let shadow_terminal =
            shadow_terminal::active_terminal::ActiveTerminal::start_with_pty(config.clone())?;
        let mut runner = Self {
            cli,
            screen,
            shadow_terminal,
        };
        let Some(input_tx) = &runner.shadow_terminal.pty_input_tx else {
            color_eyre::eyre::bail!("Shadow Terminal not setup to receive PTY input");
        };
        Self::forward_stdin(input_tx.clone());
        runner.listen_for_output().await?;

        Ok(())
    }

    /// Setup the app.
    fn setup(
        mut cli: crate::cli_args::CliArgs,
    ) -> Result<shadow_terminal::shadow_terminal::Config> {
        if cli.width.is_none() || cli.height.is_none() {
            let capabilities = shadow_terminal::termwiz::caps::Capabilities::new_from_env()?;
            let mut user_terminal =
                shadow_terminal::termwiz::terminal::SystemTerminal::new(capabilities)?;
            let user_size = user_terminal.get_screen_size()?;
            cli.width = Some((user_size.cols - 5).try_into()?);
            cli.height = Some((user_size.rows.div_euclid(2)).try_into()?);
        }

        let width = cli
            .width
            .context("No width given. This should be impossible")?;
        let height = cli
            .height
            .context("No height given. This should be impossible")?;

        let config = shadow_terminal::shadow_terminal::Config {
            width,
            height,
            command: cli.command,
            scrollback_size: cli.scrollback_size,
            ..Default::default()
        };

        Ok(config)
    }

    /// Listen for output from the shadow terminal.
    async fn listen_for_output(&mut self) -> Result<()> {
        while let Some(output) = self.shadow_terminal.surface_output_rx.recv().await {
            #[expect(
                clippy::single_match,
                reason = "I hope to add the scrollback output later"
            )]
            match output {
                shadow_terminal::output::native::Output::Diff(surface_diff) => match surface_diff {
                    shadow_terminal::output::native::SurfaceDiff::Screen(screen_diff) => {
                        self.screen.surface.add_changes(screen_diff.changes);
                        self.screen.mode = screen_diff.mode;
                        self.output()?;
                    }
                    shadow_terminal::output::native::SurfaceDiff::Scrollback(_) | _ => (),
                },
                shadow_terminal::output::native::Output::Complete(complete_surface) => {
                    match complete_surface {
                        shadow_terminal::output::native::CompleteSurface::Screen(
                            complete_screen,
                        ) => {
                            self.screen.surface = complete_screen.surface;
                            self.screen.mode = complete_screen.mode;
                            self.output()?;
                        }
                        shadow_terminal::output::native::CompleteSurface::Scrollback(_) | _ => (),
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    /// Output the contents of the shadow terminal using the configured output format.
    fn output(&mut self) -> Result<()> {
        match self.cli.output {
            #[expect(clippy::print_stdout, reason = "We gotta print out at some point")]
            crate::cli_args::OutputFormat::JSON => {
                let output = shadow_terminal::output::foreign::Output::convert_to_foreign(
                    shadow_terminal::output::native::CompleteSurface::Screen(self.screen.clone()),
                )?;
                println!("{}", serde_json::to_string(&output)?);
            }
            crate::cli_args::OutputFormat::Plain => self.print_plain(),
        }

        Ok(())
    }

    /// Crudely print out the contents of the surface in a UTF8 box.
    #[expect(clippy::print_stdout, reason = "We gotta print out at some point")]
    fn print_plain(&mut self) {
        let width = self.screen.surface.dimensions().0;
        println!("╭{}╮", "─".repeat(width));
        for line in self.screen.surface.screen_cells() {
            print!("│");
            for cell in line {
                print!("{}", cell.str());
            }
            println!("│");
        }
        println!("╰{}╯", "─".repeat(width));
        println!();
    }

    /// Forward STDIN to the shadow terminal.
    fn forward_stdin(pty_input_tx: tokio::sync::mpsc::Sender<shadow_terminal::bytes::Bytes>) {
        // The Tokio docs actually suggest using `std::thread` to listen on STDIN for interactive
        // applications.
        std::thread::spawn(move || {
            let buffer_size = 128;
            let stdin = std::io::stdin();
            let mut reader = std::io::BufReader::new(stdin);
            let mut buffer = shadow_terminal::bytes::BytesMut::zeroed(buffer_size);

            loop {
                if buffer.len() < buffer_size {
                    buffer.resize(buffer_size, 0);
                }
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        tracing::debug!("STDIN reader received EOF, exiting...");
                        break;
                    }
                    Ok(size) => {
                        let filled_data = buffer.split_to(size);
                        let frozen_bytes = filled_data.freeze();

                        if let Err(error) = pty_input_tx.try_send(frozen_bytes) {
                            tracing::error!("Couldn't forward STDIN to shadow terminal: {error:?}");
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::error!("Couldn't read from STDIN: {error:?}");
                        break;
                    }
                }
            }
        });
    }
}

#[expect(clippy::indexing_slicing, reason = "It's okay in tests")]
#[cfg(test)]
mod test {
    fn workspace_dir() -> std::path::PathBuf {
        let output = std::process::Command::new(env!("CARGO"))
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format=plain")
            .output()
            .unwrap()
            .stdout;
        let cargo_path = std::path::Path::new(std::str::from_utf8(&output).unwrap().trim());
        let workspace_dir = cargo_path.parent().unwrap().to_path_buf();
        tracing::debug!("Using workspace directory: {workspace_dir:?}");
        workspace_dir
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Just nicer like this for testing"
    )]
    fn run(
        command: Vec<&str>,
        format: crate::cli_args::OutputFormat,
    ) -> assert_cmd::assert::Assert {
        let format_as_string = format!("{format:?}").to_lowercase();
        let bin = format!("{}/target/debug/shadow-terminal", workspace_dir().display());
        let mut cmd = assert_cmd::Command::new(bin);
        let mut args = vec![
            "--width",
            "80",
            "--height",
            "10",
            "--output",
            format_as_string.as_str(),
        ];
        args.extend(command.iter());
        cmd.args(args).assert()
    }

    #[test]
    fn basic_bash_command() {
        let result = run(
            vec![
                "cat",
                format!(
                    "{}/shadow-terminal/src/tests/cat_me.txt",
                    workspace_dir().display()
                )
                .as_str(),
            ],
            crate::cli_args::OutputFormat::Plain,
        );
        result
            .success()
            .stdout(predicates::str::is_match("earth").unwrap());
    }

    #[test]
    fn check_json_output() {
        let result = run(
            vec![
                "cat",
                format!(
                    "{}/shadow-terminal/src/tests/cat_me.txt",
                    workspace_dir().display()
                )
                .as_str(),
            ],
            crate::cli_args::OutputFormat::JSON,
        );
        let all_lines = String::from_utf8(result.success().get_output().stdout.clone()).unwrap();
        let json = all_lines.lines().last().unwrap();
        let output = match serde_json::from_str::<shadow_terminal::output::foreign::Output>(json) {
            Ok(output) => output,
            Err(err) => {
                panic!("{err}\n{json}")
            }
        };
        assert_eq!(output.cells[0].text, "e".to_owned());
        assert_eq!(output.cells[4].text, "h".to_owned());

        assert_eq!(output.width, 80);
        assert_eq!(output.height, 10);

        assert_eq!(
            output.mode,
            shadow_terminal::output::native::ScreenMode::Primary
        );

        assert_eq!(
            output.cursor.shape,
            Some(shadow_terminal::termwiz::surface::CursorShape::Default)
        );
        assert_eq!(
            output.cursor.visibility,
            shadow_terminal::termwiz::surface::CursorVisibility::Visible
        );
        assert_eq!(output.cursor.position, (0, 1));
    }
}
