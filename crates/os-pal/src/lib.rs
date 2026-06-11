use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn get_config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn get_data_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.data_dir().to_path_buf())
}

pub fn get_runtime_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.runtime_dir().map(|p| p.to_path_buf()).unwrap_or_else(|| dirs.data_dir().to_path_buf()))
}

pub fn get_log_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/log/ttgtiso-desk"))
    }
    #[cfg(target_os = "windows")]
    {
        let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
            .context("Failed to get system directory paths")?;
        Ok(dirs.data_local_dir().join("logs"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
            .context("Failed to get system directory paths")?;
        Ok(dirs.cache_dir().to_path_buf())
    }
}

pub fn execute_command(program: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(program).args(args).output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
