use clap::{Parser, Subcommand};

/// Upload chunky files to Discord with webhooks
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Subcommand
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Upload a file
    Upload {
        file: String,
        #[clap(short)]
        webhook: Option<String>,
    },
    /// Download a file
    Download {
        /// URL of the index file
        url: String,
        /// The path to save the file in. If unspecified or a directory, the original filename will be used.
        #[clap(short)]
        output: Option<String>,
    },
    /// Delete a file
    Delete {
        /// Message ID of the index
        mid: u64,
        #[clap(short)]
        webhook: Option<String>,
    },
    /// Manage webhooks
    Webhook {
        #[clap(subcommand)]
        command: WebhookCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum WebhookCommand {
    /// Add a new webhook
    Add { name: String, url: String },
    /// Delete a webhook
    Delete { name: String },
    /// List webhooks
    List,
    /// Set a webhook as the default
    Default { name: String },
}
