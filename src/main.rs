use clap::Parser;
use color_eyre::Result;

mod cli;
mod commands;
mod config;
mod discord;
mod multipart;
mod read_ext;

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    let cli = cli::Args::parse();
    match cli.command {
        cli::Command::Upload { file, webhook } => commands::upload(&file, webhook.as_deref()),
        cli::Command::Download { url, output } => commands::download(&url, output.as_deref()),
        cli::Command::Delete { mid, webhook } => commands::delete(mid, webhook.as_deref()),
    }
}
