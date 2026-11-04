<#
.SYNOPSIS
  Fails (exit 1) if any hand-written TS/TSX/Rust source file exceeds the line limit
  (CLAUDE.md: "under 200 lines per file").

.DESCRIPTION
  Scans src/**/*.{ts,tsx}, src-tauri/src/**/*.rs and src-tauri/tests/**/*.rs.
  Skips src/types/generated (ts-rs output: regenerated, never edited by hand).
  Runs on Windows PowerShell 5.1, no pwsh-only syntax.

.EXAMPLE
  npm run check:loc
  powershell -File scripts/check-loc.ps1 -Max 150 -Top 10
#>
param(
  [int]$Max = 200,
  # Also list the N largest files, so creeping files are visible before they fail.
  [int]$Top = 5
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

$sets = @(
  @{ Dir = 'src';             Ext = @('.ts', '.tsx') },
  @{ Dir = 'src-tauri\src';   Ext = @('.rs') },
  @{ Dir = 'src-tauri\tests'; Ext = @('.rs') }
)
$skip = [regex]::Escape((Join-Path $root 'src\types\generated'))

$files = foreach ($s in $sets) {
  $dir = Join-Path $root $s.Dir
  if (-not (Test-Path $dir)) { continue }
  Get-ChildItem -Path $dir -Recurse -File |
    Where-Object { $s.Ext -contains $_.Extension -and $_.FullName -notmatch "^$skip" }
}

# ReadAllLines, not Get-Content: ~10x faster and counts a trailing line without '\n'
# the same way editors do.
$counted = $files | ForEach-Object {
  [pscustomobject]@{
    Lines = [System.IO.File]::ReadAllLines($_.FullName).Count
    Path  = $_.FullName.Substring($root.Length + 1)
  }
} | Sort-Object Lines -Descending

$over = @($counted | Where-Object { $_.Lines -gt $Max })

Write-Host ("Checked {0} files (limit {1} lines). Largest:" -f @($counted).Count, $Max)
$counted | Select-Object -First $Top | ForEach-Object { Write-Host ("  {0,4}  {1}" -f $_.Lines, $_.Path) }

if ($over.Count -gt 0) {
  Write-Host ''
  Write-Host ("{0} file(s) over {1} lines - split them:" -f $over.Count, $Max) -ForegroundColor Red
  $over | ForEach-Object { Write-Host ("  {0,4}  {1}" -f $_.Lines, $_.Path) -ForegroundColor Red }
  exit 1
}
Write-Host 'OK' -ForegroundColor Green
