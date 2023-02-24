mod actions;
mod arguments;

use crate::{cli, logger};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub enum Cli {
    /// Init shuffle node, see `shuffle --help` for more information
    Shuffle(arguments::Shuffle),
    /// Provide operations with UTXOs `utxo --help` for more information
    Utxo(UtxosCommand),
}

#[derive(Args)]
pub struct UtxosCommand {
    #[command(subcommand)]
    pub cmd: UtxoSubCommand,
}

#[derive(Subcommand)]
pub enum UtxoSubCommand {
    /// Return information about UTXO(s)
    Get(arguments::GetUTXOs),
}

impl Cli {
    pub async fn exec(self) -> eyre::Result<()> {
        Ok(match self {
            Self::Shuffle(args) => actions::shuffle(args).await,
            Self::Utxo(subcommand) => match subcommand.cmd {
                UtxoSubCommand::Get(args) => Ok(()),
            },
        }?)
    }
}

pub async fn run() -> eyre::Result<()> {
    logger::init(&"debug".to_string());

    cli::Cli::parse().exec().await
}
