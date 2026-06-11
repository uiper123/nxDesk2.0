# =============================================================================
#  TTGTiSO-Desk Server Agent — Windows installer
# =============================================================================
#  Installs the server agent as an auto-start Windows service that runs in the
#  background under the LocalSystem account (survives reboots and logoff).
#
#  Usage (run in an elevated / Administrator PowerShell):
#      # Install the latest release from GitHub:
#      powershell -ExecutionPolicy Bypass -File install-agent.ps1
#
#      # Install from a local binary you already built:
#      powershell -ExecutionPolicy Bypass -File install-agent.ps1 -BinaryPath .\server-agent.exe
#
#      # Uninstall:
#      powershell -ExecutionPolicy Bypass -File install-agent.ps1 -Uninstall
# =============================================================================

[CmdletBinding()]
param(
    [string]$BinaryPath = "",
    [string]$Repo = "uiper123/nxDesk2.0",
    [string]$Version = "latest",
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$ServiceName  = "TTGTiSODeskAgent"
$InstallDir   = Join-Path $env:ProgramFiles "TTGTiSO-Desk"
$ConfigDir    = Join-Path $env:ProgramData  "TTGTiSO-Desk"
$ConfigFile   = Join-Path $ConfigDir "agent.toml"
$LogDir       = Join-Path $ConfigDir "logs"
$ExeName      = "ttgtiso-desk-agent.exe"
$ExePath      = Join-Path $InstallDir $ExeName

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "This script must be run as Administrator. Right-click PowerShell and choose 'Run as administrator'."
    }
}

function Stop-And-Remove-Service {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($null -ne $svc) {
        Write-Host "Stopping existing service '$ServiceName'..."
        if ($svc.Status -ne "Stopped") {
            Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
            $svc.WaitForStatus("Stopped", (New-TimeSpan -Seconds 15)) | Out-Null
        }
        # Prefer the agent's own uninstall (cleans SCM definition), fall back to sc.exe
        if (Test-Path $ExePath) {
            & $ExePath --uninstall-service 2>$null | Out-Null
        }
        if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
            sc.exe delete $ServiceName | Out-Null
        }
        Start-Sleep -Seconds 1
    }
}

function Get-LatestAssetUrl {
    param([string]$Repo, [string]$Version)
    if ($Version -eq "latest") {
        $api = "https://api.github.com/repos/$Repo/releases/latest"
    } else {
        $api = "https://api.github.com/repos/$Repo/releases/tags/$Version"
    }
    Write-Host "Querying GitHub release metadata: $api"
    $headers = @{ "User-Agent" = "ttgtiso-desk-installer" }
    $rel = Invoke-RestMethod -Uri $api -Headers $headers
    $asset = $rel.assets | Where-Object { $_.name -match "windows" -and $_.name -match "\.exe$" } | Select-Object -First 1
    if ($null -eq $asset) {
        $asset = $rel.assets | Where-Object { $_.name -like "*server-agent*windows*" } | Select-Object -First 1
    }
    if ($null -eq $asset) {
        throw "Could not find a Windows .exe asset in the '$($rel.tag_name)' release. Build locally and pass -BinaryPath instead."
    }
    return $asset.browser_download_url
}

# ----------------------------------------------------------------------------- 
Assert-Admin

if ($Uninstall) {
    Stop-And-Remove-Service
    Write-Host "Service removed."
    Write-Host "Note: program files in '$InstallDir' and config in '$ConfigDir' were left in place."
    Write-Host "Delete them manually if you want a full uninstall."
    exit 0
}

Write-Host "=== TTGTiSO-Desk Server Agent — Windows install ==="

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir  | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir     | Out-Null

# Stop any running instance before overwriting the binary.
Stop-And-Remove-Service

# Obtain the binary.
if ($BinaryPath -ne "") {
    if (-not (Test-Path $BinaryPath)) { throw "BinaryPath '$BinaryPath' does not exist." }
    Write-Host "Copying binary from $BinaryPath"
    Copy-Item -Path $BinaryPath -Destination $ExePath -Force
} else {
    $url = Get-LatestAssetUrl -Repo $Repo -Version $Version
    Write-Host "Downloading agent from: $url"
    $tmp = Join-Path $env:TEMP $ExeName
    Invoke-WebRequest -Uri $url -OutFile $tmp -Headers @{ "User-Agent" = "ttgtiso-desk-installer" }
    Copy-Item -Path $tmp -Destination $ExePath -Force
    Remove-Item $tmp -ErrorAction SilentlyContinue
}

# Write a default config if none exists.
if (-not (Test-Path $ConfigFile)) {
    Write-Host "Writing default config to $ConfigFile"
    @"
# TTGTiSO-Desk Remote Desktop Server Agent Configuration (Windows)
bind_address = "0.0.0.0"
port = 2222

[session_limits]
max_concurrent_sessions = 4
session_timeout_seconds = 3600

[security_policy]
allow_password_auth = true
enable_audit_logs = true
"@ | Set-Content -Path $ConfigFile -Encoding UTF8
}

# Register + start the service via the agent's built-in installer (auto-start at boot).
Write-Host "Registering Windows service '$ServiceName' (auto-start)..."
& $ExePath --install-service
if ($LASTEXITCODE -ne 0) {
    throw "Service installation failed with exit code $LASTEXITCODE"
}

# Make sure the service recovers automatically if it ever crashes.
sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/5000/restart/5000 | Out-Null

Start-Sleep -Seconds 2
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($null -ne $svc) {
    Write-Host ""
    Write-Host "=== Installation complete ==="
    Write-Host "Service : $ServiceName"
    Write-Host "State   : $($svc.Status)"
    Write-Host "Binary  : $ExePath"
    Write-Host "Config  : $ConfigFile"
    Write-Host "Logs    : $LogDir"
    Write-Host "Startup : Automatic (runs in background, survives reboot/logoff)"
} else {
    throw "Service '$ServiceName' was not found after installation."
}
