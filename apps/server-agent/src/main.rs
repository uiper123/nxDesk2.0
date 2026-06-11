mod app;
mod handler;
mod platform;
mod socket;

#[cfg(target_os = "windows")]
mod service_windows;

use anyhow::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "ttgtiso-desk-agent {VERSION}\n\n\
Usage: ttgtiso-desk-agent [OPTIONS]\n\n\
Options:\n  \
-V, --version            Print version and exit\n  \
-h, --help               Print this help and exit\n  \
    --config <PATH>      Path to the agent config file\n\n\
Windows service management (Windows only):\n  \
    --install-service    Install and start the agent as a Windows service\n  \
    --uninstall-service  Stop and remove the Windows service\n  \
    --run-service        Internal: run under the Windows Service Control Manager\n"
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ttgtiso-desk-agent {VERSION}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    // --config <PATH>: make the config path available to the config loader.
    if let Some(pos) = args.iter().position(|a| a == "--config") {
        if let Some(path) = args.get(pos + 1) {
            std::env::set_var("TTGTISO_CONFIG", path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if args.iter().any(|a| a == "--install-service") {
            return service_windows::install_service();
        }
        if args.iter().any(|a| a == "--uninstall-service") {
            return service_windows::uninstall_service();
        }
        if args.iter().any(|a| a == "--run-service") {
            // Launched by the Windows Service Control Manager.
            return service_windows::run_as_service();
        }
    }

    // Default: run in the foreground (managed by systemd on Linux, or run
    // directly for development / manual use on any platform).
    run_foreground()
}

/// Run the agent event loop in the foreground on a fresh Tokio runtime.
fn run_foreground() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(app::AgentApp::run())
}
