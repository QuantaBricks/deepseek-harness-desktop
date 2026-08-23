param([Parameter(Mandatory=$true)][string]$Core,[Parameter(Mandatory=$true)][string]$Archive)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$root=(Resolve-Path $Core).Path
$links=@()
function Get-RelativePath($from,$to) {
  $fromUri=[Uri]::new(($from.TrimEnd('\\')+'\\'))
  $toUri=[Uri]::new($to)
  [Uri]::UnescapeDataString($fromUri.MakeRelativeUri($toUri).ToString()).Replace('/','\\')
}
Get-ChildItem -LiteralPath $root -Recurse -Force -Attributes ReparsePoint -ErrorAction SilentlyContinue | ForEach-Object {
  $target=[string]$_.Target
  if($target.StartsWith($root,[StringComparison]::OrdinalIgnoreCase)) {
    $links += [ordered]@{
      path=(Get-RelativePath $root $_.FullName).Replace('\\','/')
      target=Get-RelativePath (Split-Path -Parent $_.FullName) $target
    }
  }
}
$flat=Join-Path ([IO.Path]::GetTempPath()) ('dsh-core-flat-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $flat | Out-Null
try {
  robocopy $root $flat /E /XJ /XJD /XJF /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
  if($LASTEXITCODE -gt 7) { throw "Failed to stage flat core (robocopy exit $LASTEXITCODE)." }
  $json=if($links.Count){$links|ConvertTo-Json -Depth 4}else{'[]'}
  Set-Content -LiteralPath (Join-Path $flat 'links.json') -Value $json -Encoding utf8
  if(Test-Path -LiteralPath $Archive) { [IO.File]::Delete($Archive) }
  [IO.Compression.ZipFile]::CreateFromDirectory($flat,$Archive,[IO.Compression.CompressionLevel]::Optimal,$false)
} finally {
  Remove-Item -LiteralPath $flat -Recurse -Force -ErrorAction SilentlyContinue
}
