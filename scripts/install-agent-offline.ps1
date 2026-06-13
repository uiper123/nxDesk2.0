# =============================================================================
#  TTGTiSO-Desk Server Agent — Windows Offline/Air-Gapped Installer
# =============================================================================
#  Installs the server agent and configures OpenSSH entirely offline.
#
#  Usage (run in an elevated / Administrator PowerShell):
#      powershell -ExecutionPolicy Bypass -File install-agent-offline.ps1 `
#          -BinaryPath .\server-agent.exe `
#          -OpenSshZipPath .\OpenSSH-Win64.zip `
#          -SshKey "ssh-ed25519 AAAAC3NzaC1..."
# =============================================================================

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$BinaryPath,

    [string]$OpenSshZipPath = "",
    [string]$SshKey = "",
    [switch]$Unattended
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
        if (Test-Path $ExePath) {
            & $ExePath --uninstall-service 2>$null | Out-Null
        }
        if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
            sc.exe delete $ServiceName | Out-Null
        }
        Start-Sleep -Seconds 1
    }
}

# ----------------------------------------------------------------------------- 
Assert-Admin

Write-Host "=========================================================="
Write-Host "   TTGTiSO-Desk Agent Offline Setup for Windows   "
Write-Host "=========================================================="
Write-Host ""

# 1. Install OpenSSH Server Offline if not present
Write-Host "📦 Step 1: Checking OpenSSH Server..."
$sshService = Get-Service -Name sshd -ErrorAction SilentlyContinue
if ($null -eq $sshService) {
    if ($OpenSshZipPath -eq "") {
        Write-Warning "OpenSSH is not installed, and no -OpenSshZipPath was provided."
        Write-Host "Attempting native Windows capability install (requires internet or local WSUS)..."
        try {
            $sshStatus = Get-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
            if ($sshStatus.State -ne "Installed") {
                Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0 | Out-Null
            }
        } catch {
            Write-Warning "WindowsCapability failed. Attempting dism..."
            dism.exe /online /enable-feature /featurename:OpenSSH-Server-Package-Client-Package /all /norestart | Out-Null
        }
    } else {
        if (-not (Test-Path $OpenSshZipPath)) {
            throw "OpenSshZipPath '$OpenSshZipPath' does not exist."
        }
        Write-Host "Installing OpenSSH Server offline from zip: $OpenSshZipPath"
        $sshInstallDir = "C:\Program Files\OpenSSH"
        if (-not (Test-Path $sshInstallDir)) {
            New-Item -ItemType Directory -Path $sshInstallDir -Force | Out-Null
        }
        
        Write-Host "Extracting OpenSSH archive..."
        Expand-Archive -Path $OpenSshZipPath -DestinationPath $env:TEMP -Force
        
        # OpenSSH zip usually contains a subfolder OpenSSH-Win64
        $extractedPath = Join-Path $env:TEMP "OpenSSH-Win64"
        if (-not (Test-Path $extractedPath)) {
            $extractedPath = Join-Path $env:TEMP "OpenSSH"
        }
        if (-not (Test-Path $extractedPath)) {
            # Check any subdirectory inside $env:TEMP that has install-sshd.ps1
            $foundDir = Get-ChildItem -Path $env:TEMP -Filter "install-sshd.ps1" -Recurse | Select-Object -First 1
            if ($null -ne $foundDir) {
                $extractedPath = $foundDir.DirectoryName
            }
        }
        
        if (-not (Test-Path $extractedPath)) {
            throw "Could not find extracted OpenSSH directory in temp folder."
        }
        
        Copy-Item -Path "$extractedPath\*" -Destination $sshInstallDir -Recurse -Force
        Remove-Item -Path $extractedPath -Recurse -Force
        
        Write-Host "Running OpenSSH installation script..."
        Push-Location $sshInstallDir
        powershell.exe -ExecutionPolicy Bypass -File .\install-sshd.ps1
        Pop-Location
    }
} else {
    Write-Host "✅ OpenSSH Server is already installed."
}

# Ensure sshd is running and starts automatically
Write-Host "Enabling and starting OpenSSH service (sshd)..."
Set-Service -Name sshd -StartupType 'Automatic'
$sshService = Get-Service -Name sshd
if ($sshService.Status -ne "Running") {
    Start-Service sshd
}

# 2. Configure SSH Public Key
if ($SshKey -eq "" -and -not $Unattended) {
    $SshKey = Read-Host -Prompt "Please enter/paste the SSH public key (ssh-ed25519 ...) of the main client"
}

if ($SshKey -ne "") {
    Write-Host "🔑 Step 2: Configuring SSH public key authorization..."
    
    # User profile authorized_keys
    $sshDir = Join-Path $Home ".ssh"
    if (-not (Test-Path $sshDir)) {
        New-Item -ItemType Directory -Force -Path $sshDir | Out-Null
    }
    $authKeysFile = Join-Path $sshDir "authorized_keys"
    if (-not (Test-Path $authKeysFile)) {
        New-Item -ItemType File -Force -Path $authKeysFile | Out-Null
    }
    # Avoid duplicate entry
    $content = Get-Content $authKeysFile -ErrorAction SilentlyContinue
    if ($content -notcontains $SshKey) {
        Add-Content -Path $authKeysFile -Value "$SshKey"
    }
    
    # Administrators group authorized_keys (required if the user is in Admin group)
    $adminKeysFile = "C:\ProgramData\ssh\administrators_authorized_keys"
    if (Test-Path "C:\ProgramData\ssh") {
        if (-not (Test-Path $adminKeysFile)) {
            New-Item -ItemType File -Force -Path $adminKeysFile | Out-Null
        }
        $adminContent = Get-Content $adminKeysFile -ErrorAction SilentlyContinue
        if ($adminContent -notcontains $SshKey) {
            Add-Content -Path $adminKeysFile -Value "$SshKey"
        }
        # Strict ACL permissions required by sshd on Windows for administrators_authorized_keys
        icacls.exe $adminKeysFile /inheritance:r /grant "Administrators:F" /grant "SYSTEM:F" | Out-Null
    }
    Write-Host "✅ SSH public key registered."
}

# 3. Copy Agent Binary
Write-Host "📦 Step 3: Installing Agent files..."
if (-not (Test-Path $BinaryPath)) {
    throw "Provided BinaryPath '$BinaryPath' does not exist."
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir  | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir     | Out-Null

Stop-And-Remove-Service

Write-Host "Copying binary to installation directory..."
Copy-Item -Path $BinaryPath -Destination $ExePath -Force

# 4. Write Configuration
if (-not (Test-Path $ConfigFile)) {
    Write-Host "Writing default configuration..."
    $configLines = @(
        '# TTGTiSO-Desk Remote Desktop Server Agent Configuration (Windows)',
        'bind_address = "0.0.0.0"',
        'port = 2222',
        '',
        '[session_limits]',
        'max_concurrent_sessions = 4',
        'session_timeout_seconds = 3600',
        '',
        '[security_policy]',
        'allow_password_auth = true',
        'enable_audit_logs = true'
    )
    $configLines | Set-Content -Path $ConfigFile -Encoding UTF8
}

# 5. Register and Start Windows Service
Write-Host "⚙️ Step 4: Registering Windows service '$ServiceName'..."
& $ExePath --install-service
if ($LASTEXITCODE -ne 0) {
    throw "Service installation failed with exit code $LASTEXITCODE"
}

# Set recovery options
sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/5000/restart/5000 | Out-Null

Start-Sleep -Seconds 2
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($null -ne $svc) {
    Write-Host ""
    Write-Host "🎉 === Installation Complete ==="
    Write-Host "Service : $ServiceName"
    Write-Host "State   : $($svc.Status)"
    Write-Host "Path    : $ExePath"
    Write-Host "Config  : $ConfigFile"
    Write-Host "Logs    : $LogDir"
    Write-Host "================================="
} else {
    throw "Service '$ServiceName' was not found after installation."
}
