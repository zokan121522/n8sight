use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    #[serde(skip_serializing, default)]
    pub api_key: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_page_size() -> u32 {
    50
}

fn default_log_level() -> String {
    "warn".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:5678".to_string(),
            api_key: String::new(),
            project_id: None,
            page_size: default_page_size(),
            log_level: default_log_level(),
        }
    }
}

impl Config {
    pub fn load(
        url_override: Option<&str>,
        key_override: Option<&str>,
        project_override: Option<&str>,
    ) -> Result<Self> {
        let mut config = Self::default();

        if let Some(file_config) = Self::load_from_file()? {
            config.merge(file_config);
        }

        if let Ok(url) = std::env::var("N8N_API_URL") {
            config.api_url = url;
        }
        if let Ok(key) = std::env::var("N8N_API_KEY") {
            config.api_key = key;
        }
        if let Ok(project) = std::env::var("N8N_PROJECT_ID") {
            config.project_id = Some(project);
        }
        if let Ok(page_size) = std::env::var("N8N_PAGE_SIZE") {
            if let Ok(ps) = page_size.parse() {
                config.page_size = ps;
            }
        }

        if let Some(url) = url_override {
            config.api_url = url.to_string();
        }
        if let Some(key) = key_override {
            config.api_key = key.to_string();
        }
        if let Some(project) = project_override {
            config.project_id = Some(project.to_string());
        }

        config.api_url = config.api_url.trim_end_matches('/').to_string();
        Ok(config)
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("n8sight").join("config.toml"))
    }

    fn load_from_file() -> Result<Option<Self>> {
        let path = match Self::config_path() {
            Some(p) if p.exists() => p,
            _ => return Ok(None),
        };

        let contents = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self =
            toml::from_str(&contents).wrap_err("Failed to parse config file as TOML")?;

        Ok(Some(config))
    }

    fn merge(&mut self, other: Self) {
        if !other.api_url.is_empty() && other.api_url != "http://localhost:5678" {
            self.api_url = other.api_url;
        }
        if !other.api_key.is_empty() {
            self.api_key = other.api_key;
        }
        if other.project_id.is_some() {
            self.project_id = other.project_id;
        }
        if other.page_size != default_page_size() {
            self.page_size = other.page_size;
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            let hint = Self::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "config.toml".to_string());
            color_eyre::eyre::bail!(
                "No API key configured.\nSet N8N_API_KEY environment variable, or add api_key to {}",
                hint
            );
        }
        if self.api_url.is_empty() {
            color_eyre::eyre::bail!("No API URL configured.");
        }
        Ok(())
    }

    pub fn base_url(&self) -> String {
        format!("{}/api/v1", self.api_url)
    }
}
