# Installs Noob on Windows, for just you (no administrator needed).
#
# Run this in PowerShell:
#   irm https://raw.githubusercontent.com/Anand-Shekhawat/noob-cli/main/scripts/install.ps1 | iex
#
# What it does: downloads the newest Noob from GitHub, checks the download isn't
# damaged, puts noob.exe in your user Programs folder, and adds that folder to PATH
# so typing `noob` works in any new terminal window.

# To install one exact version instead of the newest: .\install.ps1 -Version v0.3.0
param([string]$Version = '')

$ErrorActionPreference = 'Stop'
$repo = 'Anand-Shekhawat/noob-cli'
$target = 'x86_64-pc-windows-msvc' # also runs on ARM Windows PCs, which can run x64 programs
# Only accept downloads from Noob's own releases on GitHub.
$trusted = "https://github.com/$repo/releases/download/"

# Older Windows PowerShell may not use modern encryption (TLS 1.2) unless asked.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

if ($Version -and $Version -notmatch '^v\d+\.\d+\.\d+$') { throw "That doesn't look like a version: $Version (try v0.3.0)" }
$which = if ($Version) { "tags/$Version" } else { 'latest' }
Write-Host 'Looking for Noob on GitHub...'
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/$which"
$asset = $release.assets | Where-Object { $_.name -like "*-$target.zip" } | Select-Object -First 1
if (-not $asset) { throw "I couldn't find a Windows download in Noob $($release.tag_name)." }
$fingerprint = $release.assets | Where-Object { $_.name -eq "$($asset.name).sha256" } | Select-Object -First 1
if (-not $fingerprint) { throw "Noob $($release.tag_name) has no checksum file, so I can't check the download." }
foreach ($file in @($asset, $fingerprint)) {
    if (-not $file.browser_download_url.StartsWith($trusted)) { throw "Unexpected download address: $($file.browser_download_url)" }
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ("noob-" + [Guid]::NewGuid())
New-Item -ItemType Directory $temp | Out-Null
try {
    $zip = Join-Path $temp $asset.name
    Write-Host "Downloading Noob $($release.tag_name)..."
    Invoke-WebRequest $asset.browser_download_url -OutFile $zip -UseBasicParsing

    # A checksum is the file's fingerprint. If it doesn't match, the download is damaged.
    $expected = ((Invoke-RestMethod $fingerprint.browser_download_url) -split '\s+')[0].ToLower()
    $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw 'The download is damaged (its checksum does not match), so I stopped.' }

    Expand-Archive $zip -DestinationPath $temp
    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\noob'
    New-Item -ItemType Directory -Force $installDir | Out-Null
    $exe = Get-ChildItem $temp -Recurse -Filter 'noob.exe' | Select-Object -First 1
    Copy-Item $exe.FullName $installDir -Force
}
finally {
    Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
}

# Add the folder to your own PATH (not the whole computer's), once.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not $userPath) { $userPath = '' }
if (($userPath -split ';') -notcontains $installDir) {
    $newPath = (($userPath.TrimEnd(';'), $installDir) | Where-Object { $_ }) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    $env:Path = "$env:Path;$installDir"
    Write-Host "Added $installDir to your PATH."
}

Write-Host "Noob $($release.tag_name) is installed. Open a new terminal window and type: noob" -ForegroundColor Green
