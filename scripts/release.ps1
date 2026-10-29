# Builds a signed release and publishes it to GitHub Releases.
# Installed apps pick it up on their next update check.
# Usage: .\scripts\release.ps1 -Version 0.2.0 -Notes "What changed"
param(
    [Parameter(Mandatory)][string]$Version,
    [string]$Notes = ''
)
$ErrorActionPreference = 'Stop'
$repo = 'invu1318/deskwidget'
$key = Join-Path $env:USERPROFILE '.tauri\deskwidget.key'
if (-not $Notes) { $Notes = "v$Version" }
Set-Location (Split-Path $PSScriptRoot -Parent)

if (git status --porcelain) { throw 'Commit or discard local changes before releasing.' }

# Version lives in package.json (tauri.conf.json reads it); keep Cargo.toml in sync.
npm version $Version --no-git-tag-version --allow-same-version | Out-Null
if ($LASTEXITCODE) { exit $LASTEXITCODE }
$cargo = Join-Path (Get-Location) 'src-tauri\Cargo.toml'
$text = [IO.File]::ReadAllText($cargo) -replace '(?m)^version = ".*"', "version = `"$Version`""
[IO.File]::WriteAllText($cargo, $text)

$env:TAURI_SIGNING_PRIVATE_KEY = [IO.File]::ReadAllText($key)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [IO.File]::ReadAllText("$key.password")
npm run tauri build
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$nsis = Join-Path (Get-Location) 'src-tauri\target\release\bundle\nsis'
$name = "DeskWidget_${Version}_x64-setup.exe"
$installer = Join-Path $nsis $name
$latest = Join-Path $nsis 'latest.json'
$manifest = [ordered]@{
    version   = $Version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = @{
        'windows-x86_64' = @{
            signature = [IO.File]::ReadAllText("$installer.sig").Trim()
            url       = "https://github.com/$repo/releases/download/v$Version/$name"
        }
    }
}
[IO.File]::WriteAllText($latest, ($manifest | ConvertTo-Json -Depth 5))

git add -A
git commit -m "v$Version"
git tag "v$Version"
git push origin HEAD --tags
if ($LASTEXITCODE) { exit $LASTEXITCODE }
gh release create "v$Version" $installer "$installer.sig" $latest --repo $repo --title "v$Version" --notes $Notes
exit $LASTEXITCODE
