use clap::Parser;

use shell::{providers::RholangFakeInterpreterProvider, run_shell, Args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let interpreter = RholangFakeInterpreterProvider::new()?;

    run_shell(args, interpreter).await
}
