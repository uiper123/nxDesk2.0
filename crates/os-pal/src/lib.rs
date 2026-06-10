use anyhow::{Result, Context};
use std::path::PathBuf;
use directories::ProjectDirs;

pub fn get_config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
        .context("Failed to get system directory paths")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn get_log_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/var/log/ttgtiso-desk"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let dirs = ProjectDirs::from("com", "ttgtiso", "desk")
            .context("Failed to get system directory paths")?;
        Ok(dirs.cache_dir().to_path_buf())
    }
}

pub fn execute_command(program: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()?;
        
    if !output.status.success() {
        anyhow::bail!("Command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
