<#
.SYNOPSIS
  Dev-time only: downloads, verifies and installs the offline sidecars OmniConvert bundles.
  The shipped app never touches the network — this runs on the developer machine.

.DESCRIPTION
  - ffmpeg + ffprobe  (BtbN/FFmpeg-Builds, immutable dated "autobuild" release, win64 GPL)
  - pandoc            (jgm/pandoc latest release, windows-x86_64 zip)
  - typst             (typst/typst latest release, x86_64-pc-windows-msvc zip)
  - pdfium.dll        (bblanchon/pdfium-binaries latest release, win-x64 tgz)

  Installs to src-tauri/binaries/<name>-x86_64-pc-windows-msvc.exe (the name Tauri's
  bundle.externalBin requires) plus a hard link at binaries/dev/<name>.exe for dev runs
  (pandoc identifies --pdf-engine by basename, so typst must also exist as "typst.exe").

  Versions + SHA-256 are pinned in scripts/sidecars.lock.json. A hash mismatch is fatal:
  we never bundle a binary we haven't verified. Use -UpdateLock to (re)resolve the latest
  releases and record their hashes (trust-on-first-use; commit the lock file afterwards).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/fetch-sidecars.ps1 -UpdateLock
  powershell -ExecutionPolicy Bypass -File scripts/fetch-sidecars.ps1 -Only ffmpeg -Force
#>
[CmdletBinding()]
param(
    [switch]$UpdateLock,
    [string[]]$Only = @('ffmpeg', 'pandoc', 'typst', 'pdfium'),
    [switch]$Force
)
# Written for Windows PowerShell 5.1 (no ternaries / ?? / -Parallel).
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # IWR's progress bar makes 5.1 downloads ~10x slower
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Root     = Split-Path -Parent $PSScriptRoot
$Bins     = Join-Path $Root 'src-tauri\binaries'
$DevBins  = Join-Path $Bins 'dev'
$Res      = Join-Path $Root 'src-tauri\resources'
$LockPath = Join-Path $PSScriptRoot 'sidecars.lock.json'
$Triple   = 'x86_64-pc-windows-msvc'
$Headers  = @{ 'User-Agent' = 'OmniConvert-fetch-sidecars'; 'Accept' = 'application/vnd.github+json' }

# name -> how to find it upstream and which files to pull out of the archive
$Sources = [ordered]@{
    ffmpeg = @{ Repo = 'BtbN/FFmpeg-Builds'; Asset = '^ffmpeg-n[\d.]+-\d+-g[0-9a-f]+-win64-gpl-[\d.]+\.zip$'; Files = @{ 'ffmpeg.exe' = 'ffmpeg'; 'ffprobe.exe' = 'ffprobe' } }
    pandoc = @{ Repo = 'jgm/pandoc';         Asset = '^pandoc-[\d.]+-windows-x86_64\.zip$';                     Files = @{ 'pandoc.exe' = 'pandoc' } }
    typst  = @{ Repo = 'typst/typst';        Asset = '^typst-x86_64-pc-windows-msvc\.zip$';                    Files = @{ 'typst.exe' = 'typst' } }
    pdfium = @{ Repo = 'bblanchon/pdfium-binaries'; Asset = '^pdfium-win-x64\.tgz$';                         Files = @{ 'pdfium.dll' = 'pdfium.dll' } }
}

function Resolve-Release($name, $src) {
    if ($name -eq 'ffmpeg') {
        # BtbN's "latest" tag is rolling; dated autobuild-* releases are immutable, so pin one.
        $releases = Invoke-RestMethod -Headers $Headers "https://api.github.com/repos/$($src.Repo)/releases?per_page=10"
        $rel = $releases | Where-Object { $_.tag_name -like 'autobuild-*' } | Select-Object -First 1
    } else {
        $rel = Invoke-RestMethod -Headers $Headers "https://api.github.com/repos/$($src.Repo)/releases/latest"
    }
    $asset = $rel.assets | Where-Object { $_.name -match $src.Asset } | Sort-Object name -Descending | Select-Object -First 1
    if (-not $asset) { throw "$name : no asset matching $($src.Asset) in $($src.Repo)@$($rel.tag_name)" }
    @{ version = $rel.tag_name; url = $asset.browser_download_url; file = $asset.name; sha256 = '' }
}

function Install-Payload($name, $src, $extractDir) {
    foreach ($entry in $src.Files.GetEnumerator()) {
        $found = Get-ChildItem -Path $extractDir -Recurse -File -Filter $entry.Key | Select-Object -First 1
        if (-not $found) { throw "$name : $($entry.Key) not found in archive" }
        if ($entry.Value -like '*.dll') {
            Copy-Item $found.FullName (Join-Path $Res $entry.Value) -Force
            Write-Host "  -> resources\$($entry.Value)"
            continue
        }
        $dest = Join-Path $Bins "$($entry.Value)-$Triple.exe"
        $dev  = Join-Path $DevBins "$($entry.Value).exe"
        Copy-Item $found.FullName $dest -Force
        if (Test-Path $dev) { Remove-Item $dev -Force }
        # Hard link: same bytes, zero extra disk (pandoc alone is ~200 MB).
        New-Item -ItemType HardLink -Path $dev -Target $dest | Out-Null
        Write-Host "  -> binaries\$($entry.Value)-$Triple.exe (+ dev\$($entry.Value).exe)"
    }
}

function Test-Ffmpeg {
    $exe = Join-Path $DevBins 'ffmpeg.exe'
    $enc = (& $exe -hide_banner -encoders 2>$null) -join "`n"
    $dec = (& $exe -hide_banner -decoders 2>$null) -join "`n"
    $missing = @()
    foreach ($e in 'libx264', 'libx265', 'libvpx-vp9', 'h264_nvenc', 'h264_amf', 'h264_qsv', 'libwebp_anim', 'libmp3lame', 'libopus', 'libvorbis') {
        if ($enc -notmatch "\b$([regex]::Escape($e))\b") { $missing += $e }
    }
    if ($dec -notmatch '\blibdav1d\b') { $missing += 'libdav1d (decoder)' }
    if ($missing.Count) { throw "ffmpeg build lacks required codecs: $($missing -join ', ')" }
    Write-Host '  ffmpeg codec check passed'
}

# ---------------------------------------------------------------------------
New-Item -ItemType Directory -Force -Path $Bins, $DevBins, $Res | Out-Null
$lock = @{}
if (Test-Path $LockPath) {
    (Get-Content $LockPath -Raw | ConvertFrom-Json).PSObject.Properties | ForEach-Object {
        $lock[$_.Name] = @{ version = $_.Value.version; url = $_.Value.url; file = $_.Value.file; sha256 = $_.Value.sha256 }
    }
}

$work = Join-Path ([IO.Path]::GetTempPath()) ("omni-sidecars-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    foreach ($name in $Sources.Keys) {
        if ($Only -notcontains $name) { continue }
        $src = $Sources[$name]
        Write-Host "[$name]"

        if ($UpdateLock -or -not $lock.ContainsKey($name)) {
            if (-not $UpdateLock) { throw "$name is not pinned in sidecars.lock.json - run with -UpdateLock first" }
            $lock[$name] = Resolve-Release $name $src
            Write-Host "  resolved $($lock[$name].version)"
        }
        $pin = $lock[$name]

        $first = @($src.Files.Values)[0]
        $installed = if ($first -like '*.dll') { Join-Path $Res $first } else { Join-Path $Bins "$first-$Triple.exe" }
        if ((Test-Path $installed) -and -not $Force -and -not $UpdateLock) { Write-Host '  already installed (use -Force to refetch)'; continue }

        # Fresh, isolated dir per download: extracted third-party files never mix with our scripts.
        $dir = Join-Path $work $name
        New-Item -ItemType Directory -Path $dir | Out-Null
        $archive = Join-Path $dir $pin.file
        Write-Host "  downloading $($pin.url)"
        Invoke-WebRequest -Headers $Headers -Uri $pin.url -OutFile $archive -UseBasicParsing

        $hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
        if ($pin.sha256 -and $pin.sha256 -ne $hash) { throw "$name : SHA-256 mismatch (expected $($pin.sha256), got $hash) - refusing to install" }
        $pin.sha256 = $hash

        $extract = Join-Path $dir 'x'
        New-Item -ItemType Directory -Path $extract | Out-Null
        if ($archive -like '*.zip') { Expand-Archive -Path $archive -DestinationPath $extract }
        else { & "$env:SystemRoot\System32\tar.exe" -xzf $archive -C $extract; if ($LASTEXITCODE) { throw "$name : tar failed" } }

        Install-Payload $name $src $extract
        if ($name -eq 'ffmpeg') { Test-Ffmpeg }
    }
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}

# Stable key order + no BOM (5.1's Set-Content -Encoding UTF8 writes one) => clean diffs.
$out = [ordered]@{}
foreach ($n in $Sources.Keys) {
    if ($lock.ContainsKey($n)) {
        $e = $lock[$n]
        $out[$n] = [ordered]@{ version = $e.version; file = $e.file; url = $e.url; sha256 = $e.sha256 }
    }
}
[IO.File]::WriteAllText($LockPath, ($out | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding $false))
Write-Host "Done. Lock file: $LockPath"
