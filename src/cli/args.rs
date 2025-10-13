use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "invoice-agent")]
#[command(about = "Automated invoice fetcher from Gmail to Google Drive", long_about = None)]
pub struct Cli {
    /// Skip Docker container execution (for internal use)
    #[arg(long, hide = true)]
    pub no_docker: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run manually (fetch invoices immediately)
    Manual {
        /// Custom date range in format YYYY-MM-DD:YYYY-MM-DD
        /// If not provided, uses previous month
        #[arg(short, long)]
        date_range: Option<String>,
    },

    /// Run in scheduled mode (only executes on configured day)
    Scheduled,

    /// Manage authentication tokens
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Re-authenticate Gmail account
    Gmail,

    /// Re-authenticate Google Drive account
    Drive,

    /// Clear all tokens (force re-authentication for both)
    Reset,
}
