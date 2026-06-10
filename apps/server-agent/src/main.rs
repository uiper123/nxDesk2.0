mod app;
mod socket;
mod handler;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Start the application runner
    app::AgentApp::run().await
}
