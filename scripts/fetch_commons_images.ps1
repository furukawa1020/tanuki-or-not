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

# Wikimedia recommends providing a descriptive User-Agent for API clients
$commonHeaders = @{ 'User-Agent' = 'tanuki-or-not-fetch-script/1.0 (contact: none)'; 'Accept' = 'application/json' }

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
        # Strategy: find category pages (ns=14) related to the query, then fetch files from those categories.
        $catSearch = [uri]::EscapeDataString($q)
        $catApi = "https://commons.wikimedia.org/w/api.php?action=query&format=json&list=search&srsearch=$catSearch&srnamespace=14&srlimit=10"
        try {
            $catResp = Invoke-RestMethod -Uri $catApi -Headers $commonHeaders -UseBasicParsing -TimeoutSec 30
        } catch {
            Write-Warning ("Failed to search categories for {0}: {1}" -f $q, ($_.Exception.Message))
            continue
        }
        if (-not $catResp.query -or -not $catResp.query.search) { continue }
        $categoryTitles = $catResp.query.search | ForEach-Object { $_.title }
        if (-not $categoryTitles -or $categoryTitles.Count -eq 0) {
            # fallback: try a direct Category:<Query> name (English)
            $categoryTitles = @("Category:$q")
        }
        # Collect file titles from two sources:
        # 1) members of discovered categories (ns=6)
        # 2) direct full-text search in file namespace (ns=6) for the query term
        $allFileTitles = @()
        foreach ($catTitle in $categoryTitles) {
            $cmApi = "https://commons.wikimedia.org/w/api.php?action=query&format=json&list=categorymembers&cmtitle=$( [uri]::EscapeDataString($catTitle) )&cmnamespace=6&cmlimit=500"
            try {
                $cmResp = Invoke-RestMethod -Uri $cmApi -Headers $commonHeaders -UseBasicParsing -TimeoutSec 30
            } catch {
                Write-Warning ("Failed to get members of {0}: {1}" -f $catTitle, ($_.Exception.Message))
                continue
            }
            if ($cmResp.query -and $cmResp.query.categorymembers) {
                $allFileTitles += ($cmResp.query.categorymembers | ForEach-Object { $_.title })
            }
            Start-Sleep -Milliseconds 200
        }

        # Direct file search (ns=6) for the query term to catch images not in nicely named categories
        $fileSearchApi = "https://commons.wikimedia.org/w/api.php?action=query&format=json&list=search&srsearch=$( [uri]::EscapeDataString($q) )&srnamespace=6&srlimit=500"
        try {
            $fileResp = Invoke-RestMethod -Uri $fileSearchApi -Headers $commonHeaders -UseBasicParsing -TimeoutSec 30
            if ($fileResp.query -and $fileResp.query.search) {
                $allFileTitles += ($fileResp.query.search | ForEach-Object { $_.title })
            }
        } catch {
            Write-Warning ("Direct file search failed for {0}: {1}" -f $q, ($_.Exception.Message))
        }

        $titles = $allFileTitles | Select-Object -Unique
        if (-not $titles -or $titles.Count -eq 0) { continue }
        # Use generator=search in file namespace directly for the query term (simpler, reliable)
        Write-Host ("Using generator=search for query '{0}'" -f $q)
        $gsrApi = "https://commons.wikimedia.org/w/api.php?action=query&format=json&generator=search&gsrsearch=$( [uri]::EscapeDataString($q) )&gsrnamespace=6&gsrlimit=500&prop=imageinfo&iiprop=url|extmetadata"
        Write-Host "Request URL: $gsrApi"
        try {
            $gsrResp = Invoke-RestMethod -Uri $gsrApi -UseBasicParsing -TimeoutSec 30
        } catch {
            Write-Warning ("generator=search failed for {0}: {1}" -f $q, ($_.Exception.Message))
            $gsrResp = $null
        }
        if ($gsrResp -and $gsrResp.query -and $gsrResp.query.pages) {
            $pageCount = ($gsrResp.query.pages.Values | Measure-Object).Count
            Write-Host ("generator=search returned {0} pages for query '{1}'" -f $pageCount, $q)
            $pages = $gsrResp.query.pages.Values | Sort-Object -Property index
            foreach ($p in $pages) {
                if ($saved -ge $PerCategory) { break }
                if (-not $p.imageinfo) { continue }
                $ii = $p.imageinfo[0]
                $license = ''
                if ($ii.extmetadata -and $ii.extmetadata.LicenseShortName) { $license = CleanHtml($ii.extmetadata.LicenseShortName.value) }
                $acceptable = $false
                if ($license -match 'Public domain' -or $license -match 'CC0' -or $license -match 'Creative Commons' -or $license -match 'CC BY' -or $license -match 'CC-BY' -or $license -match '^CC') { $acceptable = $true }
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
        Start-Sleep -Milliseconds 300
    }
    Write-Host ("Category {0}: saved {1} images" -f $key, $saved)
    if ($saved -lt $PerCategory) {
        Write-Warning "Only found $saved/$PerCategory acceptable images for $key. You may want to add more manually."
    }
}

Write-Host "Done. Check $assetDir and $licensesFile. Commit the files when ready."
