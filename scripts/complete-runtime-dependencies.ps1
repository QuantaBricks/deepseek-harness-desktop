param(
  [Parameter(Mandatory=$true)][string]$Source,
  [Parameter(Mandatory=$true)][string]$Core
)

$ErrorActionPreference='Stop'
$sourceModules=Join-Path (Resolve-Path $Source).Path 'node_modules'
$coreRoot=(Resolve-Path $Core).Path
$runtime=Join-Path $coreRoot 'harness'
$runtimeModules=Join-Path $runtime 'node_modules'
$node=Join-Path $coreRoot 'node.exe'
$seen=@{}
$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}

function Copy-Dependency([string]$name) {
  if($seen[$name] -or $name.StartsWith('@deepseek-ai/')) { return }
  $seen[$name]=$true
  $from=Join-Path $sourceModules $name
  $manifest=Join-Path $from 'package.json'
  if(!(Test-Path -LiteralPath $manifest) -and $name -eq 'zod') {
    $from=Get-ChildItem (Join-Path $sourceModules '.pnpm') -Directory -Filter 'zod@*' -ErrorAction SilentlyContinue |
      ForEach-Object { Join-Path $_.FullName 'node_modules\zod' } |
      Where-Object { Test-Path -LiteralPath (Join-Path $_ 'package.json') } |
      Select-Object -First 1
    if($from) { $manifest=Join-Path $from 'package.json' }
  }
  if(!$from -or !(Test-Path -LiteralPath $manifest)) { return }
  $to=Join-Path $runtimeModules $name
  if(Test-Path -LiteralPath $to) {
    $existing=Get-Item -LiteralPath $to -Force
    if(($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      & cmd.exe /c rmdir "$to" | Out-Null
    }
  }
  New-Item -ItemType Directory -Path $to -Force | Out-Null
  # pnpm exposes packages through junctions. Resolve that chain before
  # copying; otherwise robocopy /XJ skips the package and leaves a dangling
  # link in the release archive.
  $copyFrom=$from
  for($hop=0;$hop-lt 8;$hop++) {
    $item=Get-Item -LiteralPath $copyFrom -Force
    if(($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) { break }
    $target=$item.Target
    if($target -is [array]) { $target=$target[0] }
    if([string]::IsNullOrWhiteSpace([string]$target)) { break }
    if(-not [IO.Path]::IsPathRooted([string]$target)) {
      $target=Join-Path (Split-Path -Parent $copyFrom) ([string]$target)
    }
    $copyFrom=(Resolve-Path -LiteralPath $target).Path
  }
  robocopy $copyFrom $to /E /XD node_modules /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to copy runtime dependency $name." }
  $package=Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
  foreach($dependencies in @($package.dependencies,$package.optionalDependencies)) {
    if($null-ne $dependencies) {
      foreach($dependency in $dependencies.psobject.Properties.Name) { Copy-Dependency $dependency }
    }
  }
}

Copy-Dependency '@koromix/koffi-win32-x64'
Copy-Dependency '@img/sharp-win32-x64'
Copy-Dependency 'zod'

for($attempt=1;$attempt-le 10;$attempt++) {
  $stdout=Join-Path $tempRoot "dsh-runtime-$attempt.out.log"
  $stderr=Join-Path $tempRoot "dsh-runtime-$attempt.err.log"
  $process=Start-Process -FilePath $node -ArgumentList 'lib/bin.js','web','--host','127.0.0.1','--port','3199','--no-open' -WorkingDirectory $runtime -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
  $ready=$false
  for($tick=0;$tick-lt 120;$tick++) {
    if($process.HasExited) { break }
    try {
      $response=Invoke-WebRequest 'http://127.0.0.1:3199/' -UseBasicParsing -TimeoutSec 2
      if($response.StatusCode-eq 200) {
        Start-Sleep -Seconds 5
        if(!$process.HasExited) { $ready=$true;break }
      }
    } catch {}
    Start-Sleep -Milliseconds 250
  }
  if(!$process.HasExited) { Stop-Process -Id $process.Id -Force }
  if($ready) { return }
  $text=(Get-Content -LiteralPath $stdout -Raw -ErrorAction SilentlyContinue)+(Get-Content -LiteralPath $stderr -Raw -ErrorAction SilentlyContinue)
  $missing=[regex]::Matches($text,"Cannot find package '([^']+)'") | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
  if($missing.Count-eq 0) { throw "Harness runtime smoke test failed without a missing-package diagnostic.`n$text" }
  foreach($name in $missing) { Copy-Dependency $name }
}
throw 'Harness runtime dependency completion exceeded 10 attempts.'
