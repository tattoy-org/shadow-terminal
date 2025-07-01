//! Setup the CLI arguments.

/// Arguments to pass to the CLI binary.
#[derive(clap::Parser, Debug, Clone)]
#[command(version, about, long_about = "Shadow Terminal CLI arguments")]
pub struct CliArgs {
    /// The width of the shadow terminal.
    #[arg(long)]
    pub width: Option<u16>,

    /// The height of the shadow terminal.
    #[arg(long)]
    pub height: Option<u16>,

    /// The number of lines for the shadow terminal's scrollback buffer.
    #[arg(long, default_value = "1000")]
    pub scrollback_size: usize,

    /// The format to return the output of the shadow terminal.
    #[arg(long, value_enum, default_value_t)]
    pub output: OutputFormat,

    /// Generate the current JSON schema for serialised output.
    #[arg(long)]
    pub generate_schema: bool,

    /// The command to run in the shadow terminal.
    #[arg(env = "SHELL", default_value = "bash")]
    pub command: Vec<std::ffi::OsString>,
}

/// The available options for how shadow terminal output is sent to STDOUT
#[derive(
    serde::Serialize, serde::Deserialize, clap::ValueEnum, Default, Debug, Clone, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// A rich and structured representation of all the cells' data.
    #[default]
    JSON,
    /// Just a plain, monochrome format useful for debugging.
    Plain,
}
