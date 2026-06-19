use anyhow::{anyhow, bail, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, Password, Select};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

use crate::api;
use crate::config::{self, mask_key, Config};
use crate::providers::{self, ProviderSelection};
use crate::template::DEFAULT_TEMPLATE;

fn print_banner() {
    println!();
    println!("{}", "  ╭──────────────────────────────────────╮".cyan());
    println!("{}", "  │            commitai setup            │".cyan());
    println!("{}", "  ╰──────────────────────────────────────╯".cyan());
    println!();
    println!("  {} commit messages from any AI provider.", "Welcome!".bold().green());
    println!(
        "  {} OpenAI-compatible API works (OpenAI, OpenRouter, Groq, Ollama, ...).",
        "Any".dimmed()
    );
    println!();
}

pub async fn first_run_wizard() -> Result<Config> {
    print_banner();
    let theme = ColorfulTheme::default();

    let selection = pick_provider(&theme)?;
    let mut config = Config::from_selection(&selection);

    if selection.needs_key {
        println!();
        println!("  Get a key at: {}", selection.signup_url.underline().blue());
        let key = Password::with_theme(&theme)
            .with_prompt("Paste your API key")
            .allow_empty_password(false)
            .interact()?;
        config.provider.api_key = key;
    }

    config.provider.model = pick_model(&theme, &config.provider).await;

    let do_test = !selection.needs_key
        || Confirm::with_theme(&theme)
            .with_prompt("Test the connection now?")
            .default(true)
            .interact()?;
    if do_test {
        test_connection(&config.provider).await;
    }

    config::ensure_default_template()?;
    config::write(&config)?;
    println!();
    println!(
        "  {} You're all set! Run {} in a git repo with staged changes.",
        "Done.".bold().green(),
        "commitai".yellow()
    );
    println!(
        "  Config saved to: {}",
        config::config_path().display().to_string().dimmed()
    );
    println!();
    Ok(config)
}

fn pick_provider(theme: &ColorfulTheme) -> Result<ProviderSelection> {
    let presets = providers::all();
    let labels: Vec<&str> = presets.iter().map(|p| p.name).collect();
    let mut options = labels.clone();
    options.push("Custom (enter base URL manually)");

    let choice = Select::with_theme(theme)
        .with_prompt("Which AI provider do you want to use?")
        .items(&options)
        .default(0)
        .interact()?;

    if choice < presets.len() {
        return Ok(presets[choice].to_selection());
    }

    let base_url: String = Input::with_theme(theme)
        .with_prompt("Base URL")
        .default("https://api.openai.com/v1".into())
        .interact_text()?;
    let model: String = Input::with_theme(theme)
        .with_prompt("Model")
        .default("gpt-4o".into())
        .interact_text()?;
    Ok(ProviderSelection {
        base_url,
        model,
        needs_key: true,
        signup_url: String::new(),
    })
}

async fn pick_model(theme: &ColorfulTheme, provider: &config::Provider) -> String {
    let options = vec![
        format!("Keep current ({})", provider.model),
        "Fetch available models from provider".to_string(),
        "Type a model name".to_string(),
    ];
    let choice = match Select::with_theme(theme)
        .with_prompt("Model")
        .items(&options)
        .default(0)
        .interact()
    {
        Ok(c) => c,
        Err(_) => return provider.model.clone(),
    };

    match choice {
        0 => provider.model.clone(),
        1 => match api::list_models(&provider.base_url, &provider.api_key).await {
            Ok(models) if !models.is_empty() => {
                let idx = Select::with_theme(theme)
                    .with_prompt("Pick a model")
                    .items(&models)
                    .default(0)
                    .interact()
                    .unwrap_or(0);
                models[idx].clone()
            }
            Ok(_) => {
                println!("{}", "  No models returned.".yellow());
                provider.model.clone()
            }
            Err(e) => {
                println!("{} {}", "  Could not list models:".yellow(), e);
                provider.model.clone()
            }
        },
        _ => Input::with_theme(theme)
            .with_prompt("Model name")
            .default(provider.model.clone())
            .interact_text()
            .unwrap_or_else(|_| provider.model.clone()),
    }
}

async fn test_connection(provider: &config::Provider) {
    print!("  {} Connecting...", "›".cyan());
    use std::io::Write;
    let _ = std::io::stdout().flush();
    match api::list_models(&provider.base_url, &provider.api_key).await {
        Ok(models) => {
            println!(
                "\r  {} {} ({} models available)   ",
                "✓".green().bold(),
                "Connection OK".green(),
                models.len()
            );
            if !models.iter().any(|m| m == &provider.model) {
                println!(
                    "  {} model '{}' was not in the list — it may still work, but double-check.",
                    "!".yellow().bold(),
                    provider.model.yellow()
                );
            }
        }
        Err(e) => {
            println!("\r  {} {}   ", "✗".red().bold(), "Connection failed".red());
            println!("    {}", e.to_string().dimmed());
        }
    }
}

pub async fn config_menu() -> Result<()> {
    let theme = ColorfulTheme::default();
    loop {
        let cfg = config::read()?;
        let key_hint = mask_key(&cfg.provider.api_key);
        let options = vec![
            format!(
                "Provider      {} ({})",
                cfg.provider.base_url.dimmed(),
                cfg.provider.model.dimmed()
            ),
            format!("API key       {}", key_hint.dimmed()),
            format!("Model         {}", cfg.provider.model.dimmed()),
            format!("Prompt templates  ({})", cfg.templates.len()),
            "Test connection".to_string(),
            "Show full config".to_string(),
            "Quit".to_string(),
        ];

        let choice = Select::with_theme(&theme)
            .with_prompt("What do you want to configure?")
            .items(&options)
            .default(0)
            .interact()?;

        match choice {
            0 => {
                let selection = pick_provider(&theme)?;
                let mut c = config::read()?;
                c.provider.base_url = selection.base_url;
                c.provider.model = selection.model;
                config::write(&c)?;
            }
            1 => {
                let key = Password::with_theme(&theme)
                    .with_prompt("API key (input hidden)")
                    .allow_empty_password(true)
                    .interact()?;
                let mut c = config::read()?;
                c.provider.api_key = key;
                config::write(&c)?;
                println!("{}", "  Key updated.".green());
            }
            2 => {
                let c = config::read()?;
                let model = pick_model(&theme, &c.provider).await;
                let mut c = config::read()?;
                c.provider.model = model;
                config::write(&c)?;
            }
            3 => templates_menu(&theme, cfg)?,
            4 => {
                let c = config::read()?;
                test_connection(&c.provider).await;
            }
            5 => {
                let c = config::read()?;
                println!();
                println!("{}", toml::to_string_pretty(&c)?);
            }
            _ => return Ok(()),
        }
        println!();
    }
}

fn templates_menu(theme: &ColorfulTheme, cfg: Config) -> Result<()> {
    let names: Vec<String> = cfg.templates.keys().cloned().collect();
    let mut options: Vec<String> = names.clone();
    options.push("+ Add new template".to_string());
    options.push("Back".to_string());

    let choice = Select::with_theme(theme)
        .with_prompt("Templates")
        .items(&options)
        .default(0)
        .interact()?;

    if choice == options.len() - 1 {
        return Ok(());
    }
    if choice == options.len() - 2 {
        let name: String = Input::with_theme(theme)
            .with_prompt("New template name")
            .default("default".into())
            .interact_text()?;
        let path = config::templates_dir().join(format!("{name}.txt"));
        std::fs::create_dir_all(config::templates_dir())?;
        if !path.is_file() {
            std::fs::write(&path, DEFAULT_TEMPLATE)?;
        }
        let mut c = config::read()?;
        c.templates
            .insert(name.clone(), path.to_string_lossy().into_owned());
        config::write(&c)?;
        let c2 = config::read()?;
        if let Some(p) = c2.templates.get(&name) {
            edit_file(PathBuf::from(p))?;
        }
        return Ok(());
    }

    let name = &names[choice];
    if let Some(p) = cfg.templates.get(name) {
        edit_file(PathBuf::from(p))?;
    }
    Ok(())
}

fn edit_file(path: PathBuf) -> Result<()> {
    let editor = std::env::var("EDITOR").ok().filter(|e| !e.is_empty());
    let cmd = match editor {
        Some(e) => e,
        _ => {
            let pick = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select an editor")
                .items(&["vim", "nano", "code --wait", "cancel"])
                .default(0)
                .interact()?;
            match pick {
                0 => "vim".to_string(),
                1 => "nano".to_string(),
                2 => "code --wait".to_string(),
                _ => return Ok(()),
            }
        }
    };

    let mut parts = cmd.split_whitespace();
    let program = parts.next().ok_or_else(|| anyhow!("empty editor"))?;
    let args: Vec<&str> = parts.collect();

    let status = Command::new(program).args(&args).arg(&path).status()?;
    if !status.success() {
        bail!("editor exited with status {status}");
    }
    Ok(())
}
