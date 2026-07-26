[CmdletBinding()]
param(
    # L1/L2/L3 regression baselines by default — the only corpus entries
    # expected to exist in every development environment. Pass -Games to build
    # a subset or environment-local extras under
    # samples/local/iwanna-examples/gm8-core/.
    [string[]]$Games = @(
        'IWBT_Dife'
        'I Wanna Break Through ArioTrials'
        'I wanna be the Crimson ver.1.0'
    ),
    [switch]$SkipValidate
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

Push-Location -LiteralPath $repoRoot
try {
    foreach ($game in $Games) {
        $inputDir = Join-Path $repoRoot 'samples' 'local' 'iwanna-examples' 'gm8-core' $game
        $outputDir = Join-Path $repoRoot 'runtime' 'public' 'packages' 'gm8-core' $game

        if (-not (Test-Path -LiteralPath $inputDir)) {
            Write-Warning "skipping '$game': local sample not found at $inputDir"
            continue
        }

        Write-Host "building package: $game" -ForegroundColor Cyan
        cargo run -p iwm-cli -- build-package --input $inputDir --output $outputDir

        if (-not $SkipValidate) {
            cargo run -p iwm-cli -- validate-package --input $outputDir
        }
    }
} finally {
    Pop-Location
}
