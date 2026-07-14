[CmdletBinding()]
param(
    [string]$Tag = 'iwm-beta:0.2.0-beta.1'
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

Push-Location -LiteralPath $repoRoot
try {
    git submodule update --init --recursive
    @(
        'rust:1.93-bookworm'
        'node:22-bookworm-slim'
        'debian:bookworm-slim'
    ) | ForEach-Object {
        docker pull $_
    }
    $revision = (git rev-parse HEAD).Trim()
    docker build --build-arg "VCS_REF=$revision" --tag $Tag .
} finally {
    Pop-Location
}
