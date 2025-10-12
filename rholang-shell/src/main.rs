use anyhow::Result;
use clap::Parser;

use shell::{providers::RholangParserInterpreterProvider, run_shell, Args};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let interpreter = RholangParserInterpreterProvider::new()?;
    run_shell(args, interpreter).await
}
