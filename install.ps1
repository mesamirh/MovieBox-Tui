$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Info { param([string]$Message) Write-Host "$Message" -ForegroundColor Cyan }
function Write-Success { param([string]$Message) Write-Host "$Message" -ForegroundColor Green }
function Write-Warn { param([string]$Message) Write-Host "WARNING: $Message" -ForegroundColor Yellow }
function Write-Err { param([string]$Message) Write-Host "ERROR: $Message" -ForegroundColor Red; exit 1 }

$InstallDir = "$env:USERPROFILE\AppData\Local\MovieBox-Tui"
$ExePath = "$InstallDir\moviebox-tui.exe"
$ZipFile = "$env:TEMP\MovieBox_Windows_x64.zip"

Write-Info "Fetching latest version information..."
try {
    $Request = [System.Net.WebRequest]::Create("https://github.com/mesamirh/MovieBox-Tui/releases/latest")
    $Request.AllowAutoRedirect = $false
    $Response = $Request.GetResponse()
    $Location = $Response.Headers["Location"]
    $Version = $Location.Split('/')[-1]
    $Response.Close()
    if (-not $Version) { throw "Version not found from redirect." }
} catch {
    Write-Err "Failed to fetch latest version from GitHub API. Please check your internet connection."
}

$IsUpdate = $false
if (Test-Path $ExePath) {
    try {
        $Strings = (Select-String -Path $ExePath -Pattern "--version" -Quiet -ErrorAction SilentlyContinue)
        if ($Strings) {
            $CurrentVersionOutput = (& $ExePath --version 2>&1 | Out-String)
            if ($CurrentVersionOutput -match "moviebox-tui\s+([\d\.]+)") {
                $CurrentVersion = $matches[1]
                if ("v$CurrentVersion" -eq $Version) {
                    Write-Success "You already have the latest version ($Version) installed."
                    exit 0
                }
            }
        }
        if (-not $CurrentVersion) { $CurrentVersion = "unknown" }
        Write-Info "Updating MovieBox-TUI from v$CurrentVersion to $Version..."
        $IsUpdate = $true
        
        $RunningProcesses = Get-Process -Name "moviebox-tui" -ErrorAction SilentlyContinue
        if ($RunningProcesses) {
            Write-Info "Stopping running instances of MovieBox-Tui..."
            $RunningProcesses | Stop-Process -Force
            Start-Sleep -Seconds 1
        }
    } catch {
        Write-Info "Updating MovieBox-TUI to $Version..."
        $IsUpdate = $true
    }
} else {
    Write-Info "Installing MovieBox-TUI $Version..."
}

$Url = "https://github.com/mesamirh/MovieBox-Tui/releases/download/$Version/MovieBox_Windows_x64.zip"

Write-Info "Downloading release archive..."
try {
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile -UseBasicParsing
} catch {
    Write-Err "Download failed. Please check your internet connection."
}

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

Write-Info "Extracting files..."
try {
    Expand-Archive -Path $ZipFile -DestinationPath $InstallDir -Force
} catch {
    Write-Err "Failed to extract archive."
}

if ($IsUpdate) {
    Remove-Item $ExePath -Force -ErrorAction SilentlyContinue
}

Remove-Item $ZipFile -Force

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    Write-Info "Adding $InstallDir to PATH..."
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Warn "Please restart your PowerShell window for the PATH changes to take effect."
}

if ($IsUpdate) {
    Write-Success "Update complete! Run 'moviebox-tui' to start."
} else {
    Write-Success "Installation complete! Run 'moviebox-tui' to start."
}
