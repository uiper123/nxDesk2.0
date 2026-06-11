//! Windows service integration for the server agent.
//!
//! Provides three entry points used from `main.rs`:
//! * [`install_service`] — register the agent with the Service Control Manager
//!   (SCM) and start it, configured to auto-start at boot.
//! * [`uninstall_service`] — stop and delete the service.
//! * [`run_as_service`] — the body that runs under the SCM (invoked with
//!   `--run-service`); it hooks the service control dispatcher and runs the
//!   agent until a Stop control is received.

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::broadcast;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
    ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

const SERVICE_NAME: &str = "TTGTiSODeskAgent";
const SERVICE_DISPLAY_NAME: &str = "TTGTiSO-Desk Remote Desktop Agent";
const SERVICE_DESCRIPTION: &str =
    "Provides remote desktop screen streaming and input control for TTGTiSO-Desk.";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

/// Install the agent as an auto-start Windows service and start it.
pub fn install_service() -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("Failed to open Service Control Manager (run as Administrator)")?;

    let exe_path = std::env::current_exe().context("Failed to resolve current executable path")?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe_path,
        launch_arguments: vec![OsString::from("--run-service")],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };

    let service = match manager.create_service(
        &service_info,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(svc) => svc,
        Err(e) => {
            // If the service already exists, open it and continue (idempotent install).
            eprintln!("create_service failed ({e}); attempting to open existing service");
            manager
                .open_service(
                    SERVICE_NAME,
                    ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
                )
                .context("Failed to open existing service")?
        }
    };

    let _ = service.set_description(SERVICE_DESCRIPTION);
    service.start::<OsString>(&[]).ok();

    println!("Service '{SERVICE_NAME}' installed and started.");
    println!("It will start automatically at boot.");
    Ok(())
}

/// Stop and remove the Windows service.
pub fn uninstall_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Failed to open Service Control Manager (run as Administrator)")?;

    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .context("Failed to open service (is it installed?)")?;

    // Try to stop the service first, ignoring errors if already stopped.
    if let Ok(status) = service.query_status() {
        if status.current_state != ServiceState::Stopped {
            let _ = service.stop();
            // Give it a moment to stop.
            for _ in 0..20 {
                std::thread::sleep(Duration::from_millis(250));
                if let Ok(s) = service.query_status() {
                    if s.current_state == ServiceState::Stopped {
                        break;
                    }
                }
            }
        }
    }

    service.delete().context("Failed to delete service")?;
    println!("Service '{SERVICE_NAME}' uninstalled.");
    Ok(())
}

windows_service::define_windows_service!(ffi_service_main, service_main);

/// Entry point invoked with `--run-service`: hand control to the SCM dispatcher.
pub fn run_as_service() -> Result<()> {
    service_control_handler::register(SERVICE_NAME, |_| ServiceControlHandlerResult::NoError).ok();
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("Failed to start service dispatcher")?;
    Ok(())
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service_inner() {
        eprintln!("Service error: {e:?}");
    }
}

fn run_service_inner() -> Result<()> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // Register the control handler that responds to Stop / Shutdown.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            windows_service::service::ServiceControl::Stop
            | windows_service::service::ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            windows_service::service::ServiceControl::Interrogate => {
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)
        .context("Failed to register service control handler")?;

    let set_state = |state: ServiceState, controls: bool| ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: state,
        controls_accepted: if controls {
            windows_service::service::ServiceControlAccept::STOP
                | windows_service::service::ServiceControlAccept::SHUTDOWN
        } else {
            windows_service::service::ServiceControlAccept::empty()
        },
        exit_code: windows_service::service::ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(set_state(ServiceState::Running, true))?;

    // Bridge the std mpsc stop signal into a tokio broadcast channel.
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
    std::thread::spawn(move || {
        let _ = stop_rx.recv();
        let _ = shutdown_tx.send(());
    });

    // Run the agent on a dedicated Tokio runtime until shutdown is requested.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?;

    let result = runtime.block_on(crate::app::AgentApp::run_with_shutdown(Some(shutdown_rx)));

    status_handle.set_service_status(set_state(ServiceState::Stopped, false))?;

    result
}
