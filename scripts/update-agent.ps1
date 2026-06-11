# =============================================================================
#  TTGTiSO-Desk Server Agent — Windows self-update
# =============================================================================
#  Downloads the latest release binary, swaps it in, and restarts the service.
#  Keeps a backup of the previous binary and rolls back if the service fails
#  to start.
#
#  Usage (elevated / Administrator PowerShell):
#      powershell -ExecutionPolicy Bypass -File update-agent.ps1
# =============================================================================

[CmdletBinding()]
param(
    [string]$Repo = "uiper123/nxDesk2.0",
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$ServiceName = "TTGTiSODeskAgent"
$InstallDir  = Join-Path $env:ProgramFiles "TTGTiSO-Desk"
$ExeName     = "ttgtiso-desk-agent.exe"
$ExePath     = Join-Path $InstallDir $ExeName
$BackupPath  = "$ExePath.bak"

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "This script must be run as Administrator."
    }
}

function Get-LatestAssetUrl {
    param([string]$Repo, [string]$Version)
    if ($Version -eq "latest") {
        $api = "https://api.github.com/repos/$Repo/releases/latest"
    } else {
        $api = "https://api.github.com/repos/$Repo/releases/tags/$Version"
    }
    $headers = @{ "User-Agent" = "ttgtiso-desk-installer" }
    $rel = Invoke-RestMethod -Uri $api -Headers $headers
    $asset = $rel.assets | Where-Object { $_.name -match "windows" -and $_.name -match "\.exe$" } | Select-Object -First 1
    if ($null -eq $asset) {
        throw "No Windows .exe asset found in release '$($rel.tag_name)'."
    }
    return @{ Url = $asset.browser_download_url; Tag = $rel.tag_name }
}

Assert-Admin

if (-not (Test-Path $ExePath)) {
    throw "Agent is not installed at '$ExePath'. Run install-agent.ps1 first."
}

$current = (& $ExePath --version) 2>$null
Write-Host "Current version: $current"

$info = Get-LatestAssetUrl -Repo $Repo -Version $Version
Write-Host "Latest release: $($info.Tag)"
Write-Host "Downloading $($info.Url)"

$tmp = Join-Path $env:TEMP $ExeName
Invoke-WebRequest -Uri $info.Url -OutFile $tmp -Headers @{ "User-Agent" = "ttgtiso-desk-installer" }

Write-Host "Stopping service..."
Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
(Get-Service -Name $ServiceName).WaitForStatus("Stopped", (New-TimeSpan -Seconds 15)) | Out-Null

Write-Host "Backing up current binary..."
Copy-Item -Path $ExePath -Destination $BackupPath -Force

Write-Host "Installing new binary..."
Copy-Item -Path $tmp -Destination $ExePath -Force
Remove-Item $tmp -ErrorAction SilentlyContinue

Write-Host "Starting service..."
Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3

$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($null -eq $svc -or $svc.Status -ne "Running") {
    Write-Warning "Service did not start after update. Rolling back to previous binary."
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    Copy-Item -Path $BackupPath -Destination $ExePath -Force
    Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($svc.Status -eq "Running") {
        Write-Host "Rollback succeeded; previous version restored."
    } else {
        throw "Update failed and rollback could not start the service. Manual intervention required."
    }
    exit 1
}

Remove-Item $BackupPath -ErrorAction SilentlyContinue
$new = (& $ExePath --version) 2>$null
Write-Host ""
Write-Host "=== Update complete ==="
Write-Host "Updated to: $new"
Write-Host "Service state: $($svc.Status)"
