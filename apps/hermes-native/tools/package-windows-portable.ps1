[CmdletBinding()]
param([string]$OutputDirectory)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$manifestPath = Join-Path $repoRoot 'apps\hermes-native\Cargo.toml'
$manifestText = Get-Content -LiteralPath $manifestPath -Raw
$dockPins = @([regex]::Matches($manifestText, 'doofusdock-[\w-]+\s*=\s*\{[^}]*rev\s*=\s*"([0-9a-f]{40})"') | ForEach-Object { $_.Groups[1].Value } | Select-Object -Unique)
if ($dockPins.Count -ne 1) { throw "Expected one consistent DoofusDock source pin in $manifestPath; found $($dockPins.Count)." }
$dockPin = $dockPins[0]
$hermesPin = '8783054ef6cbafd86f10892241436862c1171a66'
$sourceCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') { throw 'Could not read the Hermes Native source commit.' }
$shortCommit = $sourceCommit.Substring(0, 12)

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot "artifacts\releases\hermes-native-windows-x64-$shortCommit"
}
if ([IO.Path]::IsPathRooted($OutputDirectory)) { $finalPath = [IO.Path]::GetFullPath($OutputDirectory) }
else { $finalPath = [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory)) }
if (Test-Path -LiteralPath $finalPath) { throw "Refusing to replace existing package directory: $finalPath" }
$parentPath = Split-Path -Parent $finalPath
$leaf = Split-Path -Leaf $finalPath
if (-not (Test-Path -LiteralPath $parentPath)) { New-Item -ItemType Directory -Path $parentPath | Out-Null }
$parentPath = (Resolve-Path -LiteralPath $parentPath).Path
$finalPath = Join-Path $parentPath $leaf
$stagingPath = Join-Path $parentPath ".$leaf.staging-$PID"
if (Test-Path -LiteralPath $stagingPath) { throw "Refusing to reuse existing staging directory: $stagingPath" }

$releaseDirectory = Join-Path $repoRoot 'target\release'
Push-Location $repoRoot
try {
    & cargo build --locked --release -p hermes-native --bins --features agent-automation
    if ($LASTEXITCODE -ne 0) { throw "Cargo release build failed with exit code $LASTEXITCODE." }
} finally { Pop-Location }

$mainExe = Join-Path $releaseDirectory 'hermes-native.exe'
$automationExe = Join-Path $releaseDirectory 'hermes-automation.exe'
$pdfiumDll = Join-Path $releaseDirectory 'pdfium-8125.dll'
$pdfiumSource = Join-Path $repoRoot 'apps\hermes-native\assets\pdfium\chromium-8125\bin\pdfium.dll'
foreach ($required in @($mainExe, $automationExe, $pdfiumDll, $pdfiumSource)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required release file is missing: $required" }
}
$sourcePdfiumHash = (Get-FileHash -LiteralPath $pdfiumSource -Algorithm SHA256).Hash
$builtPdfiumHash = (Get-FileHash -LiteralPath $pdfiumDll -Algorithm SHA256).Hash
if ($sourcePdfiumHash -ne $builtPdfiumHash) { throw "Built PDFium hash differs from pinned source asset: $builtPdfiumHash != $sourcePdfiumHash" }

New-Item -ItemType Directory -Path $stagingPath | Out-Null
$noticeRoot = Join-Path $stagingPath 'THIRD_PARTY_NOTICES'
$pdfiumNotice = Join-Path $noticeRoot 'PDFium-chromium-8125'
$firaNotice = Join-Path $noticeRoot 'FiraCode'
$jetBrainsNotice = Join-Path $noticeRoot 'JetBrainsMono'
$doofusDockNotice = Join-Path $noticeRoot 'DoofusDock'
$commonmarkNotice = Join-Path $noticeRoot 'egui-commonmark-backend'
$agentDirectory = Join-Path $stagingPath 'agent'
New-Item -ItemType Directory -Path $pdfiumNotice, $firaNotice, $jetBrainsNotice, $doofusDockNotice, $commonmarkNotice, $agentDirectory | Out-Null

Copy-Item -LiteralPath $mainExe -Destination (Join-Path $stagingPath 'hermes-native.exe')
Copy-Item -LiteralPath $automationExe -Destination (Join-Path $stagingPath 'hermes-automation.exe')
Copy-Item -LiteralPath $pdfiumDll -Destination (Join-Path $stagingPath 'pdfium-8125.dll')
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\agent\hermes-page.mjs') -Destination (Join-Path $agentDirectory 'hermes-page.mjs')
$pdfiumAssets = Join-Path $repoRoot 'apps\hermes-native\assets\pdfium\chromium-8125'
Copy-Item -LiteralPath (Join-Path $pdfiumAssets 'LICENSE') -Destination $pdfiumNotice
Copy-Item -LiteralPath (Join-Path $pdfiumAssets 'licenses') -Destination $pdfiumNotice -Recurse
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\assets\fonts\FiraCode\LICENSE.txt') -Destination $firaNotice
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\assets\fonts\JetBrainsMono\OFL.txt') -Destination $jetBrainsNotice
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\vendor\doofusdock-core\LICENSE') -Destination (Join-Path $doofusDockNotice 'core-LICENSE')
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\vendor\doofusdock-presentation\LICENSE') -Destination (Join-Path $doofusDockNotice 'presentation-LICENSE')
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\vendor\doofusdock-render-egui\LICENSE') -Destination (Join-Path $doofusDockNotice 'render-egui-LICENSE')
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\vendor\egui_commonmark_backend\LICENSE-MIT') -Destination $commonmarkNotice
Copy-Item -LiteralPath (Join-Path $repoRoot 'apps\hermes-native\vendor\egui_commonmark_backend\LICENSE-APACHE') -Destination $commonmarkNotice
$nativeLicenseNote = 'Cargo metadata declares Hermes Native license = MIT; this checkout has no standalone Hermes Native license text file. This note is not a substitute for that license.'
Set-Content -LiteralPath (Join-Path $noticeRoot 'Hermes-Native-license-status.txt') -Value $nativeLicenseNote -Encoding utf8

$readme = @"
Hermes Native portable package for Windows x64

Source commit: $sourceCommit
Pinned Hermes source: $hermesPin
Pinned DoofusDock: $dockPin

Start the UI with hermes-native.exe. This package does not include Hermes
profiles, credentials, Gateway state, or the Hermes Python runtime. To connect
to a local Hermes Gateway, install/configure Hermes separately and ensure its
hermes command is available on PATH. The Hermes Desktop window is not required.

PDF preview uses the adjacent pdfium-8125.dll. Font assets are embedded in the
executables; their licenses are in THIRD_PARTY_NOTICES. PDFium and its upstream
notices are also included there.

Agent-operated UI support:
- hermes-automation.exe --native-window --stdio starts the off-screen native UI
automation process with an isolated temporary profile and no Gateway.
- agent/hermes-page.mjs provides the Playwright-shaped client. Use Node.js to run
it; pass explicit command and cwd paths because this package is not a Cargo
workspace. The client uses Node built-ins; Bun has not been release validated.
- Do not use the offline automation process as proof of live Gateway behavior.

No Hermes user data is bundled. Keep the executables and pdfium-8125.dll in the
same directory. See PACKAGE.json for file hashes and source provenance.
"@
Set-Content -LiteralPath (Join-Path $stagingPath 'README.txt') -Value $readme -Encoding utf8

$fileNames = @('hermes-native.exe', 'hermes-automation.exe', 'pdfium-8125.dll', 'agent\hermes-page.mjs')
$fileRecords = @()
foreach ($relative in $fileNames) {
    $path = Join-Path $stagingPath $relative
    $fileRecords += [ordered]@{
        path = $relative.Replace('\', '/')
        length = (Get-Item -LiteralPath $path).Length
        sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$packageMetadata = [ordered]@{
    schema_version = 1
    product = 'Hermes Native'
    architecture = 'x86_64-pc-windows-msvc'
    source_commit = $sourceCommit
    hermes_source = $hermesPin
    doofusdock_source = $dockPin
    built_at_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    files = $fileRecords
    excludes_user_profiles_and_credentials = $true
}
$metadataPath = Join-Path $stagingPath 'PACKAGE.json'
$metadataTemp = "$metadataPath.tmp"
$packageMetadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $metadataTemp -Encoding utf8
$null = Get-Content -LiteralPath $metadataTemp -Raw | ConvertFrom-Json
Move-Item -LiteralPath $metadataTemp -Destination $metadataPath

foreach ($relative in $fileNames) {
    if (-not (Test-Path -LiteralPath (Join-Path $stagingPath $relative) -PathType Leaf)) { throw "Package validation failed; missing $relative" }
}
$packagedPdfiumHash = (Get-FileHash -LiteralPath (Join-Path $stagingPath 'pdfium-8125.dll') -Algorithm SHA256).Hash
if ($packagedPdfiumHash -ne $sourcePdfiumHash) { throw 'Package PDFium hash changed during copy.' }
$noticePaths = @(
    (Join-Path $pdfiumNotice 'LICENSE'),
    (Join-Path $pdfiumNotice 'licenses\pdfium.txt'),
    (Join-Path $firaNotice 'LICENSE.txt'),
    (Join-Path $jetBrainsNotice 'OFL.txt'),
    (Join-Path $doofusDockNotice 'core-LICENSE'),
    (Join-Path $doofusDockNotice 'presentation-LICENSE'),
    (Join-Path $doofusDockNotice 'render-egui-LICENSE'),
    (Join-Path $commonmarkNotice 'LICENSE-MIT'),
    (Join-Path $commonmarkNotice 'LICENSE-APACHE'),
    (Join-Path $noticeRoot 'Hermes-Native-license-status.txt')
)
foreach ($notice in $noticePaths) {
    if (-not (Test-Path -LiteralPath $notice -PathType Leaf)) { throw "Package validation failed; missing third-party notice: $notice" }
}

[IO.Directory]::Move($stagingPath, $finalPath)
Write-Output "Portable Hermes Native package created: $finalPath"
Write-Output "Source commit: $sourceCommit"
Write-Output "PDFium SHA-256: $packagedPdfiumHash"
