mod cli;
mod logger;
mod service;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
