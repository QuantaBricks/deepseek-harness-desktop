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
  # pnpm deploy may materialize a workspace package as a flat directory at the
  # root while the complete package (including nested node_modules) lives in
  # .pnpm. Point the root package back to that graph before archiving.
  $encoded=$package.name.Replace('/','+')
  $store=Get-ChildItem (Join-Path $runtimeModules '.pnpm') -Directory -Filter ($encoded+'@*') -ErrorAction SilentlyContinue |
    ForEach-Object { Join-Path $_.FullName ('node_modules\'+$package.name) } |
    Where-Object { Test-Path -LiteralPath (Join-Path $_ 'package.json') } |
    Select-Object -First 1
  if($store -and (Test-Path -LiteralPath $target)) {
    $existing=Get-Item -LiteralPath $target -Force
    if(($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) {
      Remove-Item -LiteralPath $target -Recurse -Force
      & cmd.exe /c mklink /J "$target" "$store" | Out-Null
    }
    continue
  }
  if(Test-Path -LiteralPath $target) { continue }
  New-Item -ItemType Directory -Path $target -Force | Out-Null
  robocopy $file.DirectoryName $target /E /XD node_modules src test tests coverage /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to copy workspace runtime $($package.name)." }
}
