# Fetch vetted images from Wikimedia Commons using their API
# Downloads public-domain / CC0 images for specified queries into tanuki-quiz-rust/public/assets
# Usage: run in repository root: powershell -ExecutionPolicy Bypass -File .\scripts\fetch_commons_images.ps1

Param(
    [int]$PerCategory = 10
)

$categories = @(
    @{ key = 'tanuki'; queries = @('tanuki', 'Nyctereutes procyonoides', 'raccoon dog') },
    @{ key = 'anaguma'; queries = @('anaguma', 'badger', 'Meles meles') },
    @{ key = 'hakubishin'; queries = @('hakubishin', 'masked palm civet', 'Paguma larvata') }
)

$assetDir = Join-Path -Path (Get-Location) -ChildPath 'tanuki-quiz-rust/public/assets'
if (-Not (Test-Path $assetDir)) { New-Item -ItemType Directory -Path $assetDir -Force | Out-Null }

$licensesFile = Join-Path -Path (Get-Location) -ChildPath 'tanuki-quiz-rust/LICENSES.md'
if (-Not (Test-Path $licensesFile)) { "# Image attribution and licenses`n" | Out-File -FilePath $licensesFile -Encoding UTF8 }

Function CleanHtml([string]$s) {
    if (-not $s) { return '' }
    return ($s -replace '<[^>]+>', '')
}

Write-Host "Starting image fetch: $PerCategory per category"

foreach ($cat in $categories) {
    $key = $cat.key
    $saved = 0
    $seenUrls = @{}
    foreach ($q in $cat.queries) {
        if ($saved -ge $PerCategory) { break }
        Write-Host "Searching Commons for '$q' (category $key)"
        $api = "https://commons.wikimedia.org/w/api.php?action=query&format=json&generator=search&gsrsearch=$( [uri]::EscapeDataString($q) )&gsrnamespace=6&gsrlimit=50&prop=imageinfo&iiprop=url|extmetadata"
        try {
            $resp = Invoke-RestMethod -Uri $api -UseBasicParsing -TimeoutSec 30
        } catch {
            Write-Warning ("Failed to query API for {0}: {1}" -f $q, ($_.Exception.Message))
            continue
        }
        if (-not $resp.query) { continue }
        $pages = $resp.query.pages.Values | Sort-Object -Property index
        foreach ($p in $pages) {
            if ($saved -ge $PerCategory) { break }
            $ii = $p.imageinfo[0]
            if (-not $ii) { continue }
            $license = ''
            if ($ii.extmetadata -and $ii.extmetadata.LicenseShortName) { $license = CleanHtml($ii.extmetadata.LicenseShortName.value) }
            $acceptable = $false
            if ($license -match 'Public domain' -or $license -match 'CC0' -or $license -match 'Creative Commons 0') { $acceptable = $true }
            if (-not $acceptable) { continue }
            $url = $ii.url
            if ($seenUrls.ContainsKey($url)) { continue }
            $seenUrls[$url] = $true
            $ext = [System.IO.Path]::GetExtension($url)
            if (-not $ext) { $ext = '.jpg' }
            $index = $saved + 1
            if ($index -lt 10) { $num = "0$index" } else { $num = "$index" }
            $filename = "${key}${num}${ext}"
            $outfile = Join-Path $assetDir $filename
            Write-Host ("Downloading {0} -> {1} (license: {2})" -f $url, $outfile, $license)
            try {
                Invoke-WebRequest -Uri $url -OutFile $outfile -UseBasicParsing -TimeoutSec 60
            } catch {
                Write-Warning ("Failed to download {0}: {1}" -f $url, ($_.Exception.Message))
                continue
            }
            # record license/attribution
            $title = $p.title
            $author = ''
            if ($ii.extmetadata -and $ii.extmetadata.Artist) { $author = CleanHtml($ii.extmetadata.Artist.value) }
            $desc = ''
            if ($ii.extmetadata -and $ii.extmetadata.Description) { $desc = CleanHtml($ii.extmetadata.Description.value) }
            $titleEsc = $title -replace ' ', '_'
            $line = "- public/assets/$filename — Source: https://commons.wikimedia.org/wiki/$titleEsc — License: $license — Author: $author"
            if ($desc -and $desc.Trim().Length -gt 0) { $line = $line + " — Description: $desc" }
            Add-Content -Path $licensesFile -Value $line -Encoding UTF8
            $saved++
            if ($saved -ge $PerCategory) { break }
        }
    }
    Write-Host ("Category {0}: saved {1} images" -f $key, $saved)
    if ($saved -lt $PerCategory) {
        Write-Warning "Only found $saved/$PerCategory acceptable images for $key. You may want to add more manually."
    }
}

Write-Host "Done. Check $assetDir and $licensesFile. Commit the files when ready."
