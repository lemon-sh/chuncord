use clap::Parser;
use color_eyre::Result;

mod cli;
mod commands;
mod config;
mod discord;
mod multipart;

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    let cli = cli::Args::parse();
    match cli.command {
        cli::Command::Upload { file, webhook } => commands::upload(&file, webhook.as_deref()),
        cli::Command::Download { url, output } => commands::download(&url, output.as_deref()),
        cli::Command::Delete { mid, webhook } => commands::delete(mid, webhook.as_deref()),
        cli::Command::Webhook { command } => match command {
            cli::WebhookCommand::Add { name, url } => commands::add_webhook(name, url),
            cli::WebhookCommand::Delete { name } => commands::del_webhook(name),
            cli::WebhookCommand::List => commands::list_webhooks(),
            cli::WebhookCommand::Default { name } => commands::default_webhook(name),
        },
    }
}
