param(
    [string]$RackForgeRoot = (Join-Path $PSScriptRoot '../../rackforge'),
    [string]$StoreExecutable,
    [string]$Output,
    [string]$Toolchain
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$cargoToolchain = @()
if ($Toolchain) { $cargoToolchain = @("+$Toolchain") }
if (-not $Output) { $Output = Join-Path $repo 'artifacts/RF-EPiano-0.1.5.rfplugin' }
$Output = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
if (-not $Output.EndsWith('.rfplugin')) { throw 'Output must end in .rfplugin.' }
if (Test-Path -LiteralPath $Output) { throw "Package already exists: $Output" }
New-Item -ItemType Directory -Path (Split-Path -Parent $Output) -Force | Out-Null
if ($StoreExecutable) {
    $StoreExecutable = (Resolve-Path -LiteralPath $StoreExecutable).Path
} else {
    $RackForgeRoot = (Resolve-Path -LiteralPath $RackForgeRoot).Path
}

Push-Location -LiteralPath $repo
try {
    # Give this build a known target directory even if a contributor normally
    # shares CARGO_TARGET_DIR with the adjacent RackForge checkout.
    & cargo @cargoToolchain build --locked --release -p rackforge-rf-epiano `
        --target-dir (Join-Path $repo 'target') --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'Building the RF-EPiano component failed.' }
    $package = Join-Path $repo 'package'
    $component = Join-Path $repo 'target/wasm32-unknown-unknown/release/rackforge_rf_epiano.wasm'
    if ($StoreExecutable) {
        & $StoreExecutable pack-wasm $package $component $Output
    } else {
        # Use the host checkout as cwd so its toolchain and cargo config apply.
        Push-Location -LiteralPath $RackForgeRoot
        try {
            & cargo @cargoToolchain run --locked --release -p rackforge-store -- `
                pack-wasm $package $component $Output
        } finally {
            Pop-Location
        }
    }
    if ($LASTEXITCODE -ne 0) { throw 'Packing the RF-EPiano package failed.' }
} finally {
    Pop-Location
}
