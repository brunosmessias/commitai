use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "commitai",
    version,
    about = "Let any AI write commit messages for you in lazygit"
)]
pub struct Cli {
    /// Prompt template to use (by name, as defined in config)
    #[arg(long)]
    pub template: Option<String>,

    /// How many commit messages to suggest. The output is ALWAYS exactly this
    /// many lines (or fewer if the model failed), so the lazygit menu gets a
    /// stable contract regardless of what the model wants to add.
    #[arg(long, default_value_t = 3)]
    pub n: usize,

    /// Output style. The system prompt is tailored to match:
    ///   gitmoji      — `<emoji> <description>` (default for new installs)
    ///   conventional — `<type>: <description>`
    ///   custom       — follow whatever the user template says
    #[arg(long, default_value = "gitmoji")]
    pub format: String,

    /// Print extra debug information
    #[arg(long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Configure commitai (no argument opens the interactive menu)
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Print the path to the config file
    Path,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show the current configuration (API key is masked)
    Show,
    /// Set a value: `set base_url <url>` | `set api_key <key>` | `set model <name>` | `set format <gitmoji|conventional|custom>`
    Set { key: String, value: String },
}
