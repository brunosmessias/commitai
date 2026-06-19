#[derive(Clone, Copy)]
pub struct ProviderPreset {
    pub name: &'static str,
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub needs_key: bool,
    pub signup_url: &'static str,
}

/// Owned selection produced by the UI (works for both presets and custom entries).
pub struct ProviderSelection {
    pub base_url: String,
    pub model: String,
    pub needs_key: bool,
    pub signup_url: String,
}

impl ProviderPreset {
    pub fn to_selection(self) -> ProviderSelection {
        ProviderSelection {
            base_url: self.base_url.to_string(),
            model: self.default_model.to_string(),
            needs_key: self.needs_key,
            signup_url: self.signup_url.to_string(),
        }
    }

    pub const fn openai() -> Self {
        Self {
            name: "OpenAI",
            base_url: "https://api.openai.com/v1",
            default_model: "gpt-4o",
            needs_key: true,
            signup_url: "https://platform.openai.com/api-keys",
        }
    }
}

pub fn all() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset {
            name: "OpenAI",
            base_url: "https://api.openai.com/v1",
            default_model: "gpt-4o",
            needs_key: true,
            signup_url: "https://platform.openai.com/api-keys",
        },
        ProviderPreset {
            name: "OpenRouter (all models: GPT, Claude, Gemini, ...)",
            base_url: "https://openrouter.ai/api/v1",
            default_model: "openai/gpt-4o-mini",
            needs_key: true,
            signup_url: "https://openrouter.ai/keys",
        },
        ProviderPreset {
            name: "Groq (fast, free tier)",
            base_url: "https://api.groq.com/openai/v1",
            default_model: "llama-3.3-70b-versatile",
            needs_key: true,
            signup_url: "https://console.groq.com/keys",
        },
        ProviderPreset {
            name: "Together AI",
            base_url: "https://api.together.xyz/v1",
            default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            needs_key: true,
            signup_url: "https://api.together.xyz/settings/api-keys",
        },
        ProviderPreset {
            name: "DeepSeek",
            base_url: "https://api.deepseek.com",
            default_model: "deepseek-chat",
            needs_key: true,
            signup_url: "https://platform.deepseek.com/api_keys",
        },
        ProviderPreset {
            name: "Ollama (local, no key)",
            base_url: "http://localhost:11434/v1",
            default_model: "llama3.1",
            needs_key: false,
            signup_url: "https://ollama.com",
        },
        ProviderPreset {
            name: "LM Studio (local, no key)",
            base_url: "http://localhost:1234/v1",
            default_model: "local-model",
            needs_key: false,
            signup_url: "https://lmstudio.ai",
        },
    ]
}
