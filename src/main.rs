mod api;
mod cli;
mod config;
mod git;
mod output;
mod providers;
mod template;
mod ui;

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser};
use owo_colors::OwoColorize;

use crate::cli::{Cli, Command, ConfigAction};
use crate::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            let mut chain = e.chain();
            chain.next();
            for cause in chain {
                eprintln!("  {} {}", "caused by:".dimmed(), cause);
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Some(Command::Path) => {
            println!("{}", config::config_path().display());
            Ok(())
        }
        Some(Command::Config { action }) => handle_config(action).await,
        None => generate(args.template, args.n, args.format, args.verbose).await,
    }
}

async fn handle_config(action: Option<ConfigAction>) -> Result<()> {
    match action {
        None => {
            if !config::exists() {
                ui::first_run_wizard().await?;
            } else {
                ui::config_menu().await?;
            }
            Ok(())
        }
        Some(ConfigAction::Show) => {
            let cfg = config::read()?;
            let mut display = cfg.clone();
            display.provider.api_key = config::mask_key(&display.provider.api_key);
            println!("{}", toml::to_string_pretty(&display)?);
            Ok(())
        }
        Some(ConfigAction::Set { key, value }) => {
            let mut cfg = config::read()?;
            match key.as_str() {
                "base_url" => cfg.provider.base_url = value,
                "api_key" => cfg.provider.api_key = value,
                "model" => cfg.provider.model = value,
                "format" => {
                    if template::Style::parse(&value).is_none() {
                        bail!(
                            "unknown format '{value}'. Valid: gitmoji, conventional, custom"
                        );
                    }
                    cfg.style = Some(value);
                }
                other => {
                    let _ = Cli::command().print_help();
                    bail!(
                        "unknown config key '{other}'. Valid: base_url, api_key, model, format"
                    );
                }
            }
            config::write(&cfg)?;
            println!("{}", "  Saved.".green());
            Ok(())
        }
    }
}

async fn generate(
    template_name: Option<String>,
    n: usize,
    format_arg: String,
    verbose: bool,
) -> Result<()> {
    let cfg = if !config::exists() {
        ui::first_run_wizard().await?
    } else {
        config::read()?
    };
    config::ensure_default_template()?;

    if verbose {
        eprintln!(
            "{} provider={} model={} n={}",
            "debug".dimmed(),
            cfg.provider.base_url,
            cfg.provider.model,
            n
        );
    }

    validate_api_key(&cfg)?;

    let diff = git::staged_diff()?;
    if diff.trim().is_empty() {
        bail!("no staged changes to commit. Stage files with `git add` first.");
    }

    // Resolve style: --format flag wins, else config setting, else gitmoji.
    let style = template::Style::parse(&format_arg)
        .unwrap_or_else(|| cfg.effective_style());

    // Render the user-facing template body (the file the user can edit) and
    // build the deterministic system prompt around it.
    let template_body = render_template(&cfg, template_name.as_deref(), &diff, verbose)?;
    let system = template::build_system_prompt(n, style);
    let user = template::build_user_prompt(&template_body, &diff, n);

    if verbose {
        eprintln!(
            "{} style={:?}\n{} --- system prompt ---\n{}\n--- end system ---\n",
            "debug".dimmed(),
            style,
            "debug".dimmed(),
            system
        );
    }

    let raw = api::generate_with_system(
        &cfg.provider.base_url,
        &cfg.provider.api_key,
        &cfg.provider.model,
        system,
        user,
    )
    .await?;

    if verbose {
        eprintln!(
            "{} --- raw model output ---\n{}\n--- end raw ---\n",
            "debug".dimmed(),
            raw
        );
    }

    // The whole point of this rewrite: never trust the model's framing. Even
    // a chatty model that adds preambles, <think> blocks, and closings is
    // reduced to a clean `1.\n2.\n3.` block here.
    let cleaned = output::extract_commits(&raw, n);

    if cleaned.is_empty() {
        bail!(
            "The model returned no parseable commit messages. Raw output:\n{}",
            truncate(&raw, 600)
        );
    }

    if cleaned.len() < n {
        eprintln!(
            "{} model returned only {} of {} requested commit messages",
            "warning:".yellow().bold(),
            cleaned.len(),
            n
        );
    }

    println!("{}", output::format_for_lazygit(&cleaned));
    Ok(())
}

fn validate_api_key(cfg: &Config) -> Result<()> {
    if cfg.provider.api_key.is_empty() && is_local(&cfg.provider.base_url) {
        return Ok(());
    }
    if cfg.provider.api_key.is_empty() {
        bail!(
            "no API key configured. Run {} to set one up.",
            "commitai config".yellow()
        );
    }
    Ok(())
}

fn is_local(base_url: &str) -> bool {
    let u = base_url.to_lowercase();
    u.contains("localhost") || u.contains("127.0.0.1") || u.contains("0.0.0.0")
}

fn render_template(cfg: &Config, name: Option<&str>, diff: &str, verbose: bool) -> Result<String> {
    let name = name.unwrap_or("default");
    let path = cfg
        .templates
        .get(name)
        .cloned()
        .unwrap_or_else(|| config::default_template_path().to_string_lossy().into_owned());

    if !std::path::Path::new(&path).is_file() {
        config::ensure_default_template()?;
    }

    if verbose {
        eprintln!("{} using template '{}' ({})", "debug".dimmed(), name, path);
    }

    let raw = std::fs::read_to_string(&path)?;
    Ok(raw.replace("{{diff}}", diff))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
