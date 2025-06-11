use clap::Parser;

use shell::providers::FakeInterpreterProvider;
use shell::{run_shell, Args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let interpreter = FakeInterpreterProvider;
    
    run_shell(args, interpreter).await
}