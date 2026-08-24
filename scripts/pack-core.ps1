param([Parameter(Mandatory=$true)][string]$Core,[Parameter(Mandatory=$true)][string]$Archive)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$root=(Resolve-Path $Core).Path
$links=@()
$externalLinks=@()
function Get-RelativePath($from,$to) {
  $fromUri=[Uri]::new(($from.TrimEnd([char]92)+[char]92))
  $toUri=[Uri]::new($to)
  [Uri]::UnescapeDataString($fromUri.MakeRelativeUri($toUri).ToString()).Replace('/','\')
}
Get-ChildItem -LiteralPath $root -Recurse -Force -Attributes ReparsePoint -ErrorAction SilentlyContinue |
  Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 } |
  ForEach-Object {
    $target=(Get-Item -LiteralPath $_.FullName -Force).Target
    if($target -is [array]) { $target=$target[0] }
    if(-not [string]::IsNullOrWhiteSpace([string]$target)) {
      if(-not [IO.Path]::IsPathRooted([string]$target)) {
        $target=Join-Path (Split-Path -Parent $_.FullName) ([string]$target)
      }
      if(Test-Path -LiteralPath $target) {
        $target=(Resolve-Path -LiteralPath $target).Path
        if($target.StartsWith($root,[StringComparison]::OrdinalIgnoreCase)) {
          $links += [ordered]@{
            path=(Get-RelativePath $root $_.FullName).Replace('\\','/')
            target=Get-RelativePath (Split-Path -Parent $_.FullName) $target
          }
        } else {
          $externalLinks += [ordered]@{
            path=(Get-RelativePath $root $_.FullName).Replace('\\','/')
            target=$target
          }
        }
      }
    }
  }
$flat=Join-Path ([IO.Path]::GetTempPath()) ('dsh-core-flat-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $flat | Out-Null
try {
  robocopy $root $flat /E /XJ /XJD /XJF /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to stage flat core (robocopy exit $LASTEXITCODE)." }
  foreach($link in $externalLinks) {
    $destination=Join-Path $flat $link.path
    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
    if((Get-Item -LiteralPath $link.target -Force).PSIsContainer) {
      robocopy $link.target $destination /E /XJ /XJD /XJF /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
      if($LASTEXITCODE -gt 7) { throw "Failed to materialize external runtime link $($link.path)." }
    } else {
      Copy-Item -LiteralPath $link.target -Destination $destination -Force
    }
  }
  # Restore root pnpm package links that deploy materialized as flat
  # directories. Workspace packages need their nested node_modules graph;
  # without these links Node resolves the package itself but not its deps.
  $runtimeModules=Join-Path $flat 'harness\node_modules'
  $rootPackages=@(Get-ChildItem $runtimeModules -Directory -Force -ErrorAction SilentlyContinue | Where-Object Name -notin @('.pnpm','.bin'))
  foreach($scope in @($rootPackages | Where-Object Name -like '@*')) {
    $rootPackages += Get-ChildItem $scope.FullName -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object {
      [pscustomobject]@{ FullName=$_.FullName; Name=($scope.Name+'/'+$_.Name) }
    }
  }
  foreach($packageDir in $rootPackages) {
    $packageName=[string]$packageDir.Name
    if($packageName -in @('@deepseek-ai','@opentelemetry')) { continue }
    $packagePath=$packageDir.FullName
    $item=Get-Item -LiteralPath $packagePath -Force
    if(($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { continue }
    $encoded=$packageName.Replace('/','+')
    $store=Get-ChildItem (Join-Path $runtimeModules '.pnpm') -Directory -Filter ($encoded+'@*') -ErrorAction SilentlyContinue |
      ForEach-Object { Join-Path $_.FullName ('node_modules\'+$packageName) } |
      Where-Object { Test-Path -LiteralPath (Join-Path $_ 'package.json') } |
      Select-Object -First 1
    if(!$store) {
      $store=Get-ChildItem (Join-Path $runtimeModules '.pnpm') -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName ('node_modules\'+$packageName) } |
        Where-Object { Test-Path -LiteralPath (Join-Path $_ 'package.json') } |
        Select-Object -First 1
    }
    if(!$store) { continue }
    Remove-Item -LiteralPath $packagePath -Recurse -Force
    & cmd.exe /c mklink /J "$packagePath" "$store" | Out-Null
    $links += [ordered]@{
      path=(Get-RelativePath $flat $packagePath).Replace('\\','/')
      target=Get-RelativePath (Split-Path -Parent $packagePath) $store
    }
  }
  $json=if($links.Count){$links|ConvertTo-Json -Depth 4}else{'[]'}
  Set-Content -LiteralPath (Join-Path $flat 'links.json') -Value $json -Encoding utf8
  if(Test-Path -LiteralPath $Archive) { [IO.File]::Delete($Archive) }
  [IO.Compression.ZipFile]::CreateFromDirectory($flat,$Archive,[IO.Compression.CompressionLevel]::Fastest,$false)
} finally {
  Remove-Item -LiteralPath $flat -Recurse -Force -ErrorAction SilentlyContinue
}
