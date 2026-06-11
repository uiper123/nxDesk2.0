mod app;
mod handler;
mod socket;

use anyhow::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ttgtiso-desk-agent {}", VERSION);
        return Ok(());
    }

    // Start the application runner
    app::AgentApp::run().await
}
