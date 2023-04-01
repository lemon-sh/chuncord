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
        url: String,
        #[clap(short)]
        output: Option<String>,
    },
    /// Delete a file
    Delete {
        mid: u64,
        #[clap(short)]
        webhook: Option<String>,
    },
}
