# =============================================================================
#  TTGTiSO-Desk Server Agent — Windows uninstaller
# =============================================================================
#  Stops and removes the agent Windows service. Optionally deletes program
#  files and configuration.
#
#  Usage (elevated / Administrator PowerShell):
#      powershell -ExecutionPolicy Bypass -File uninstall-agent.ps1
#      powershell -ExecutionPolicy Bypass -File uninstall-agent.ps1 -Purge
# =============================================================================

[CmdletBinding()]
param(
    [switch]$Purge
)

$ErrorActionPreference = "Stop"

$ServiceName = "TTGTiSODeskAgent"
$InstallDir  = Join-Path $env:ProgramFiles "TTGTiSO-Desk"
$ConfigDir   = Join-Path $env:ProgramData  "TTGTiSO-Desk"
$ExePath     = Join-Path $InstallDir "ttgtiso-desk-agent.exe"

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "This script must be run as Administrator."
    }
}

Assert-Admin

$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($null -ne $svc) {
    if ($svc.Status -ne "Stopped") {
        Write-Host "Stopping service '$ServiceName'..."
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        $svc.WaitForStatus("Stopped", (New-TimeSpan -Seconds 15)) | Out-Null
    }
    if (Test-Path $ExePath) {
        & $ExePath --uninstall-service 2>$null | Out-Null
    }
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        sc.exe delete $ServiceName | Out-Null
    }
    Write-Host "Service '$ServiceName' removed."
} else {
    Write-Host "Service '$ServiceName' is not installed."
}

if ($Purge) {
    Write-Host "Purging program files and configuration..."
    if (Test-Path $InstallDir) { Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue }
    if (Test-Path $ConfigDir)  { Remove-Item -Recurse -Force $ConfigDir  -ErrorAction SilentlyContinue }
    Write-Host "Removed '$InstallDir' and '$ConfigDir'."
} else {
    Write-Host "Program files in '$InstallDir' and config in '$ConfigDir' were left in place (use -Purge to delete)."
}

Write-Host "Uninstall complete."
