# Pull DraftKings My Bets into Bettor Desktop with as little manual work as possible.
#
# What this does:
#   1. Copies tools/draftkings-my-bets.js to your clipboard
#   2. Opens My Bets in the default browser
#   3. Waits for draftkings-my-bets-*.json in Downloads (after you paste in Console)
#   4. Imports into the local bet log and starts the app

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Script = Join-Path $Root "tools\draftkings-my-bets.js"
$Db = Join-Path $env:APPDATA "com.bettorcalculator.desktop\betlog.sqlite3"
$Downloads = [Environment]::GetFolderPath("UserProfile") + "\Downloads"

if (-not (Test-Path $Script)) {
	Write-Error "Missing $Script"
}

Get-Process bettor-desktop -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

Set-Clipboard -Value (Get-Content $Script -Raw)
Start-Process "https://sportsbook.draftkings.com/mybets"

Write-Host ""
Write-Host "DraftKings My Bets is open and the export script is on your clipboard."
Write-Host "In the browser: F12 -> Console -> Ctrl+V -> Enter"
Write-Host "Scroll My Bets first so every card is loaded."
Write-Host ""
Write-Host "Waiting up to 3 minutes for draftkings-my-bets-*.json in Downloads..."

$deadline = (Get-Date).AddMinutes(3)
$jsonPath = $null

while ((Get-Date) -lt $deadline) {
	$candidate = Get-ChildItem $Downloads -Filter "draftkings-my-bets-*.json" -ErrorAction SilentlyContinue |
		Sort-Object LastWriteTime -Descending |
		Select-Object -First 1

	if ($candidate -and $candidate.LastWriteTime -gt (Get-Date).AddMinutes(-5)) {
		$jsonPath = $candidate.FullName
		break
	}
	Start-Sleep -Seconds 2
}

if (-not $jsonPath) {
	Write-Host "No fresh export found. Importing the screenshot sample bet only."
	$jsonPath = Join-Path $Root "fixtures\draftkings-sample.json"
}

Push-Location $Root
try {
	cargo run --quiet --example import_bets -- $jsonPath $Db
	if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
	Write-Host ""
	Write-Host "Starting Bettor Desktop..."
	pnpm tauri dev
}
finally {
	Pop-Location
}
