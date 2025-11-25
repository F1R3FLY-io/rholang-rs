use anyhow::Result;
use clap::Parser;

use rholang_shell::{
    providers::RholangCompilerInterpreterProvider,
    run_shell, Args,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let interpreter = RholangCompilerInterpreterProvider::new()?;
    run_shell(args, interpreter).await
}
