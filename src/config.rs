use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::providers::{ProviderPreset, ProviderSelection};
use crate::template::Style;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: Provider,
    #[serde(default)]
    pub templates: BTreeMap<String, String>,
    /// Default output style. Overridable per-invocation via `--format`.
    /// `None` is treated as `Gitmoji` so existing configs without this key
    /// keep working.
    #[serde(default)]
    pub style: Option<String>,
}

impl Config {
    pub fn effective_style(&self) -> Style {
        self.style
            .as_deref()
            .and_then(Style::parse)
            .unwrap_or(Style::Gitmoji)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
        }
    }
}

impl Config {
    pub fn from_preset(preset: &ProviderPreset) -> Self {
        Self::from_selection(&preset.to_selection())
    }

    pub fn from_selection(sel: &ProviderSelection) -> Self {
        let mut templates = BTreeMap::new();
        templates.insert(
            "default".to_string(),
            default_template_path().to_string_lossy().into_owned(),
        );

        Self {
            provider: Provider {
                base_url: sel.base_url.clone(),
                api_key: String::new(),
                model: sel.model.clone(),
            },
            templates,
            style: None,
        }
    }
}

pub fn default_template_path() -> PathBuf {
    templates_dir().join("default.txt")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("commitai")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn templates_dir() -> PathBuf {
    config_dir().join("templates")
}

pub fn exists() -> bool {
    config_path().is_file()
}

pub fn read() -> Result<Config> {
    if !exists() {
        return Ok(Config::from_preset(&ProviderPreset::openai()));
    }
    let raw = fs::read_to_string(config_path())
        .with_context(|| format!("Failed to read {}", config_path().display()))?;
    let config: Config = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", config_path().display()))?;
    Ok(config)
}

pub fn write(config: &Config) -> Result<()> {
    fs::create_dir_all(config_dir())?;
    let raw = toml::to_string_pretty(config)?;
    fs::write(config_path(), raw)?;
    Ok(())
}

pub fn ensure_default_template() -> Result<()> {
    fs::create_dir_all(templates_dir())?;
    let p = default_template_path();
    if !p.is_file() {
        fs::write(&p, crate::template::DEFAULT_TEMPLATE)?;
    }
    Ok(())
}

pub fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return "not set".to_string();
    }
    let len = key.len();
    if len <= 6 {
        return "••••".to_string();
    }
    format!("{}••••{}", &key[..3], &key[len - 3..])
}
