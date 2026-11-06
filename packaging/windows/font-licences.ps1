# Keep the upstream licence texts intact, including the copyright notices above the OFL.
function Write-FontLicences([string] $Destination, [string] $InterLicence, [string] $EvoFontsDir) {
  if ($env:EVO_FONTS_REQUIRED -and -not $EvoFontsDir) { throw 'EVO_FONTS_REQUIRED is set but EVO_FONTS_DIR is missing' }
  $parts = [Collections.Generic.List[string]]::new()
  $parts.Add("EvoLight font licences`n")
  $parts.Add([IO.File]::ReadAllText($InterLicence))
  if ($EvoFontsDir) {
    $manifest = Join-Path $EvoFontsDir 'fonts\manifest.txt'
    $licences = Get-Content -LiteralPath $manifest | ForEach-Object {
      if ($_.Trim() -and -not $_.Trim().StartsWith('#')) {
        $fields = $_ -split '\|'
        if ($fields.Count -lt 6) { throw "Malformed font manifest line: $_" }
        $fields[5].Trim()
      }
    } | Sort-Object -Unique
    foreach ($licence in $licences) {
      $parts.Add("`n--- $licence ---`n")
      $parts.Add([IO.File]::ReadAllText((Join-Path $EvoFontsDir $licence)))
    }
  }
  [IO.File]::WriteAllText($Destination, ($parts -join "`n"), [Text.UTF8Encoding]::new($false))
}
