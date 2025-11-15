# Download images from source.unsplash.com for given queries and save to tanuki-quiz-rust/public/assets
# Usage: powershell -ExecutionPolicy Bypass -File .\scripts\fetch_unsplash_images.ps1 -PerCategory 30
Param(
    [int]$PerCategory = 30,
    [int]$Width = 1200,
    [int]$Height = 800
)

$categories = @(
    @{ key='tanuki'; query='tanuki' },
    @{ key='anaguma'; query='badger' },
    @{ key='hakubishin'; query='civet' }
)

$assetDir = Join-Path (Get-Location) 'tanuki-quiz-rust/public/assets'
if (-Not (Test-Path $assetDir)) { New-Item -ItemType Directory -Path $assetDir -Force | Out-Null }
$licensesFile = Join-Path (Get-Location) 'tanuki-quiz-rust/LICENSES.md'
if (-Not (Test-Path $licensesFile)) { "# Image attribution and licenses`n" | Out-File -FilePath $licensesFile -Encoding UTF8 }

Write-Host "Downloading $PerCategory images per category from Unsplash source (this may take a while)"

foreach ($cat in $categories) {
    $key = $cat.key
    $query = $cat.query
    $saved = 0
    $attempts = 0
    $seen = @{}
    while ($saved -lt $PerCategory -and $attempts -lt ($PerCategory * 6)) {
        $attempts++
        $url = "https://source.unsplash.com/${Width}x${Height}/?${query}"
        try {
            # perform request and allow following redirects to get final image
            $resp = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 30 -MaximumRedirection 10
            $final = $null
            if ($resp.BaseResponse -and $resp.BaseResponse.ResponseUri) { $final = $resp.BaseResponse.ResponseUri.AbsoluteUri }
            if (-not $final -and $resp.Headers -and $resp.Headers.Location) { $final = $resp.Headers.Location }
            if (-not $final) { $final = $url }
        } catch {
            Write-Warning ("Request failed for {0}: {1}" -f $url, ($_.Exception.Message))
            continue
        }
        if ($seen.ContainsKey($final)) { continue }
        $seen[$final] = $true

        # choose extension
        $ext = [System.IO.Path]::GetExtension($final)
        if (-not $ext -or $ext.Length -gt 5) { $ext = '.jpg' }
        $index = $saved + 1
        $num = if ($index -lt 10) { "00$index" } elseif ($index -lt 100) { "0$index" } else { "$index" }
        $filename = "${key}${num}${ext}"
        $outfile = Join-Path $assetDir $filename
        Write-Host ("Downloading {0} -> {1}" -f $final, $outfile)
        try {
            Invoke-WebRequest -Uri $final -OutFile $outfile -UseBasicParsing -TimeoutSec 60
        } catch {
            Write-Warning ("Failed to download {0}: {1}" -f $final, ($_.Exception.Message))
            continue
        }
        # record license line (Unsplash source page)
        $line = "- public/assets/$filename — Source: $final — Provider: Unsplash"
        Add-Content -Path $licensesFile -Value $line -Encoding UTF8
        $saved++
        Start-Sleep -Milliseconds 500
    }
    Write-Host ("Category {0}: saved {1} images (attempts={2})" -f $key, $saved, $attempts)
}
Write-Host "Done. Check tanuki-quiz-rust/public/assets and tanuki-quiz-rust/LICENSES.md. Commit when ready."