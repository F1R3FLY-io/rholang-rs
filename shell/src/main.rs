use clap::Parser;

use shell::{providers::RholangParserInterpreterProvider, run_shell, Args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let interpreter = RholangParserInterpreterProvider::new()?;
    run_shell(args, interpreter).await
}
