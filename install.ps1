Write-Host "Installing MovieBox-Tui..." -ForegroundColor Cyan

$Url = "https://github.com/mesamirh/MovieBox-Tui/releases/latest/download/MovieBox_Windows_x64.zip"
$InstallDir = "$env:USERPROFILE\AppData\Local\MovieBox-Tui"
$ZipFile = "$env:TEMP\MovieBox_Windows_x64.zip"

Write-Host "Downloading latest release..."
Invoke-WebRequest -Uri $Url -OutFile $ZipFile

if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

Write-Host "Extracting..."
Expand-Archive -Path $ZipFile -DestinationPath $InstallDir -Force
Rename-Item -Path "$InstallDir\MovieBox.exe" -NewName "moviebox-tui.exe" -Force -ErrorAction SilentlyContinue

Remove-Item $ZipFile -Force

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    Write-Host "Adding to PATH..."
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Host "Warning: Please restart your PowerShell window for the PATH changes to take effect." -ForegroundColor Yellow
}

Write-Host "Success! You can now run 'moviebox-tui' from anywhere." -ForegroundColor Green
