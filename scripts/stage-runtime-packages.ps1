param(
  [Parameter(Mandatory=$true)][string]$Source,
  [Parameter(Mandatory=$true)][string]$Runtime
)

$ErrorActionPreference='Stop'
$sourceRoot=(Resolve-Path $Source).Path
$runtimeRoot=(Resolve-Path $Runtime).Path
$runtimeModules=Join-Path $runtimeRoot 'node_modules'

$workspacePackages=Get-ChildItem -LiteralPath $sourceRoot -Filter package.json -Recurse |
  Where-Object {
    $_.FullName -notmatch '\\node_modules\\' -and
    $_.FullName -match '\\(apps|packages|vendor)\\' -and
    $_.FullName -notmatch '\\(test-support|examples)\\'
  }

foreach($file in $workspacePackages) {
  $package=Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
  if([string]::IsNullOrWhiteSpace($package.name)) { continue }
  $target=Join-Path $runtimeModules $package.name
  # pnpm deploy already provides these workspace packages as junctions with
  # their nested node_modules graph. Do not overwrite that graph with a flat
  # directory, or Node's package resolution breaks after extraction.
  if(Test-Path -LiteralPath $target) { continue }
  New-Item -ItemType Directory -Path $target -Force | Out-Null
  robocopy $file.DirectoryName $target /E /XD node_modules src test tests coverage /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to copy workspace runtime $($package.name)." }
}
