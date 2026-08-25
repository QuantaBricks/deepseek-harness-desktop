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
  if(Test-Path -LiteralPath $target) {
    $existing=Get-Item -LiteralPath $target -Force
    if(($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      & cmd.exe /c rmdir "$target" | Out-Null
    } else {
      continue
    }
  }
  New-Item -ItemType Directory -Path $target -Force | Out-Null
  robocopy $file.DirectoryName $target /E /XD node_modules src test tests coverage /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to copy workspace runtime $($package.name)." }
}

# pnpm deploy can omit hoisted third-party packages that are only reached
# through materialized workspace packages. Reconstruct the external dependency
# closure from the deployed workspace package manifests.
$sourceHoisted=Join-Path $sourceRoot 'node_modules\.pnpm\node_modules'
if(Test-Path -LiteralPath $sourceHoisted) {
  $visited=@{}
  function Copy-ExternalPackage([string]$packageName) {
    if([string]::IsNullOrWhiteSpace($packageName) -or $visited[$packageName]) { return }
    if($packageName.StartsWith('node:') -or $packageName.StartsWith('workspace:')) { return }
    $visited[$packageName]=$true
    $entry=Join-Path $sourceHoisted $packageName
    if(-not (Test-Path -LiteralPath $entry)) { return }
    $destination=Join-Path $runtimeModules $packageName
    $target=(Get-Item -LiteralPath $entry -Force).Target
    if($target -is [array]) { $target=$target[0] }
    if(-not $target -or -not (Test-Path -LiteralPath $target)) { return }
    $existing=Get-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
    if($existing -and (($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
      & cmd.exe /c rmdir "$destination" | Out-Null
      $existing=$null
    }
    if(-not $existing) {
      New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
      robocopy $target $destination /E /XJ /XJD /XJF /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
      if($LASTEXITCODE -gt 7) { throw "Failed to copy external runtime package $packageName." }
    }
    $manifest=Join-Path $destination 'package.json'
    if(Test-Path -LiteralPath $manifest) {
      $meta=Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
      foreach($field in @('dependencies','optionalDependencies','peerDependencies')) {
        $group=$meta.$field
        if($group) { foreach($dep in $group.PSObject.Properties.Name) { Copy-ExternalPackage $dep } }
      }
    }
  }
  $runtimePackageManifests=Get-ChildItem (Join-Path $runtimeModules '@deepseek-ai') -Directory -Force -ErrorAction SilentlyContinue |
    ForEach-Object { Get-ChildItem $_.FullName -Filter package.json -File -Recurse -Force -ErrorAction SilentlyContinue }
  foreach($manifest in $runtimePackageManifests) {
    $meta=Get-Content -LiteralPath $manifest.FullName -Raw | ConvertFrom-Json
    foreach($field in @('dependencies','optionalDependencies','peerDependencies')) {
      $group=$meta.$field
      if($group) { foreach($dep in $group.PSObject.Properties.Name) { Copy-ExternalPackage $dep } }
    }
  }
}
