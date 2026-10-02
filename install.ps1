$ErrorActionPreference = "Stop"

$APP_NAME = "anesis"
$REPO = "anesis-dev/anesis-cli"
if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Host "Unsupported architecture: anesis ships 64-bit Windows binaries only." -ForegroundColor Red
    exit 1
}
$ARCH = "x86_64"

Write-Host "Fetching latest version..." -ForegroundColor Cyan

try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $LATEST_VERSION = $release.tag_name
} catch {
    Write-Host "Failed to fetch latest version: $_" -ForegroundColor Red
    exit 1
}

if ([string]::IsNullOrEmpty($LATEST_VERSION)) {
    Write-Host "Failed to fetch latest version" -ForegroundColor Red
    exit 1
}

Write-Host "Installing $APP_NAME $LATEST_VERSION..." -ForegroundColor Green

$ASSET = "${APP_NAME}-windows-${ARCH}.zip"
$RELEASE_BASE_URL = "https://github.com/$REPO/releases/download/$LATEST_VERSION"
$DOWNLOAD_URL = "$RELEASE_BASE_URL/$ASSET"
$TMP_DIR = Join-Path $env:TEMP ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $TMP_DIR | Out-Null

Write-Host "Downloading from $DOWNLOAD_URL..." -ForegroundColor Cyan

try {
    $zipPath = Join-Path $TMP_DIR $ASSET
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $zipPath

    # Verify the archive against the release SHA256SUMS before extracting it.
    Write-Host "Verifying checksum..." -ForegroundColor Cyan
    $sumsPath = Join-Path $TMP_DIR "SHA256SUMS"
    Invoke-WebRequest -Uri "$RELEASE_BASE_URL/SHA256SUMS" -OutFile $sumsPath

    $expected = $null
    foreach ($line in Get-Content $sumsPath) {
        $fields = $line -split '\s+', 2
        if ($fields.Count -eq 2 -and $fields[1].Trim() -eq $ASSET) {
            $expected = $fields[0].Trim().ToLowerInvariant()
            break
        }
    }
    if (-not $expected) {
        throw "No checksum for $ASSET in SHA256SUMS"
    }

    $actual = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        throw "Checksum mismatch for ${ASSET}: expected $expected, got $actual"
    }
    Write-Host "✓ Checksum verified" -ForegroundColor Green

    Expand-Archive -Path $zipPath -DestinationPath $TMP_DIR -Force

    $INSTALL_DIR = Join-Path $env:USERPROFILE ".local\bin"
    if (-not (Test-Path $INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $INSTALL_DIR | Out-Null
    }

    $exeName = "$APP_NAME.exe"
    $sourcePath = Join-Path $TMP_DIR $exeName
    $destPath = Join-Path $INSTALL_DIR $exeName

    if (Test-Path $destPath) {
        Remove-Item $destPath -Force
    }

    Move-Item -Path $sourcePath -Destination $destPath -Force

    Write-Host ""
    Write-Host "✓ $APP_NAME installed successfully to $destPath" -ForegroundColor Green
    Write-Host ""

    $userPath = (Get-Item HKCU:\Environment).GetValue('Path', '', 'DoNotExpandEnvironmentNames')
    $pathEntries = @()
    if ($userPath) {
        $pathEntries = $userPath -split ';' | Where-Object { $_ -ne '' }
    }
    if ($pathEntries -notcontains $INSTALL_DIR) {
        Write-Host "Adding $INSTALL_DIR to PATH..." -ForegroundColor Yellow
        $newPath = if ($userPath) { "$userPath;$INSTALL_DIR" } else { $INSTALL_DIR }
        Set-ItemProperty -Path HKCU:\Environment -Name Path -Value $newPath -Type ExpandString
        Write-Host "✓ PATH updated. Please restart your terminal." -ForegroundColor Green
    } else {
        Write-Host "$INSTALL_DIR is already in your PATH" -ForegroundColor Green
    }

    Write-Host "Configuring PowerShell completions..." -ForegroundColor Cyan
    try {
        & $destPath completions powershell
        if ($LASTEXITCODE -ne 0) {
            throw "anesis exited with code $LASTEXITCODE"
        }
    } catch {
        Write-Host "PowerShell completions were not configured automatically: $_" -ForegroundColor Yellow
        Write-Host "Run manually: & '$destPath' completions powershell" -ForegroundColor Yellow
    }

} catch {
    Write-Host "Installation failed: $_" -ForegroundColor Red
    exit 1
} finally {
    if (Test-Path $TMP_DIR) {
        Remove-Item -Path $TMP_DIR -Recurse -Force
    }
}

Write-Host ""
Write-Host "You can now run: $APP_NAME" -ForegroundColor Cyan
