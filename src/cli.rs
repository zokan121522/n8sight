use clap::Parser;

/// n8sight (n8s) — A terminal dashboard for monitoring n8n workflows.
#[derive(Parser, Debug)]
#[command(name = "n8s", version, about, long_about = None)]
pub struct Cli {
    /// n8n instance URL (overrides N8N_API_URL env var)
    #[arg(long)]
    pub url: Option<String>,

    /// n8n API key (overrides N8N_API_KEY env var)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Default project ID filter
    #[arg(long)]
    pub project: Option<String>,

    /// Use mock data (no n8n connection required)
    #[arg(long, default_value_t = false)]
    pub mock: bool,
}
