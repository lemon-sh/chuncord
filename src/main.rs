use clap::Parser;
use color_eyre::Result;

mod cli;
mod discord;
mod config;
mod read_ext;
mod commands;

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default().display_env_section(false).install()?;
    let cli = cli::Args::parse();
    match cli.command {
        cli::Command::Upload { file, webhook } => commands::upload(&file, webhook.as_deref()),
        cli::Command::Download { url, output } => commands::download(&url, output.as_deref()),
        cli::Command::Delete { mid, webhook } => commands::delete(mid, webhook.as_deref()),
    }
}
