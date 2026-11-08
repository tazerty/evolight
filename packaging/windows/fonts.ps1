# Font setup for local packaging; dot-sourced by package.ps1.
function Initialize-EvoFonts([string] $WorkspaceRoot) {
  $fontDir = $env:EVO_FONTS_DIR
  if (-not $fontDir) { $fontDir = Join-Path $WorkspaceRoot '../evo-fonts' }
  if (-not [IO.Path]::IsPathRooted($fontDir)) { $fontDir = Join-Path $WorkspaceRoot $fontDir }

  if (-not $env:EVO_FONTS_DIR -and -not (Test-Path -LiteralPath $fontDir)) {
    # Same revision as release.yml and packaging/fonts.sh. Existing checkouts stay untouched.
    $revision = 'b076417b5880bd4154bd22f00695b1d28c6bea83'
    Write-Output "==> downloading evo-fonts ($revision) to $fontDir"
    & git clone --no-checkout https://github.com/tazerty/evo-fonts $fontDir
    if ($LASTEXITCODE -ne 0) { throw 'Could not download evo-fonts; provide a complete checkout with EVO_FONTS_DIR' }
    & git -C $fontDir checkout --detach $revision
    if ($LASTEXITCODE -ne 0) { throw 'Could not check out evo-fonts; provide a complete checkout with EVO_FONTS_DIR' }
  }

  $manifest = Join-Path $fontDir 'fonts/manifest.txt'
  if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
    throw "Missing $manifest; set EVO_FONTS_DIR to a complete evo-fonts checkout"
  }
  $env:EVO_FONTS_DIR = (Resolve-Path -LiteralPath $fontDir).Path
  $env:EVO_FONTS_REQUIRED = '1'
  Write-Output "==> embedding fonts from $env:EVO_FONTS_DIR"
}
