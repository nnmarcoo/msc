$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot  = Resolve-Path (Join-Path $ScriptDir "..")
$VendorDir = Join-Path $RepoRoot "vendor"
$BinDir    = Join-Path $VendorDir "bin"

$YtDlpUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$YtDlpExe = Join-Path $BinDir "yt-dlp.exe"

if (Test-Path $YtDlpExe) {
    Write-Host "yt-dlp already present at vendor/bin - updating it in place."
    & $YtDlpExe -U
} else {
    Write-Host "Downloading yt-dlp from GitHub releases..."
    Invoke-WebRequest -Uri $YtDlpUrl -OutFile $YtDlpExe
    Write-Host "yt-dlp installed to vendor/bin"
}

$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$BinDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
    Write-Host "Added vendor/bin to your user Path."
}
$env:Path = "$env:Path;$BinDir"

[System.Environment]::SetEnvironmentVariable("VERSE_YT_DLP_PATH", $YtDlpExe, "User")
$env:VERSE_YT_DLP_PATH = $YtDlpExe

Write-Host ""
Write-Host "Done. yt-dlp: $YtDlpExe"
& $YtDlpExe --version
Write-Host ""
Write-Host "Build with: cargo build --features explore"
Write-Host "Downloading from YouTube is against its terms of service; the"
Write-Host "explore feature is off by default for that reason."
