param(
    [string]$AppName = "tanuki-quiz-rust",
    [string]$Region = "nrt",
    [int]$VolumeSizeGB = 3,
    [string]$AdminToken
)

function ExitWithError($msg){
    Write-Error $msg
    exit 1
}

if (-not (Get-Command flyctl -ErrorAction SilentlyContinue)) {
    ExitWithError "flyctl が見つかりません。https://fly.io/docs/hands-on/install-flyctl/ を参照してインストールしてください。"
}

# check auth
$authCheck = & flyctl auth token 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "flyctl にログインしていません。ブラウザで認証するには以下を実行してください:"
    Write-Host "  flyctl auth login"
    ExitWithError "ログインが必要です。ログイン後にこのスクリプトを再実行してください。"
}

Write-Host "アプリ存在確認: $AppName"
& flyctl apps show $AppName 1>$null 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "アプリが存在しないため作成します: $AppName (region: $Region)"
    & flyctl apps create $AppName --region $Region
    if ($LASTEXITCODE -ne 0) { ExitWithError "アプリ作成に失敗しました。" }
} else {
    Write-Host "アプリは既に存在します: $AppName"
}

# ensure volume 'assets' exists
Write-Host "永続ボリューム 'assets' の確認"
$volListJson = & flyctl volumes list --app $AppName --json 2>$null
if ($LASTEXITCODE -ne 0) { $volListJson = "" }

$hasAssets = $false
if ($volListJson -ne "") {
    try { $vols = $volListJson | ConvertFrom-Json } catch { $vols = @() }
    foreach ($v in $vols) {
        if ($v.name -eq "assets") { $hasAssets = $true }
    }
}

if (-not $hasAssets) {
    Write-Host "'assets' ボリュームが見つかりません。作成します (size: ${VolumeSizeGB}GB)。"
    & flyctl volumes create assets --size $VolumeSizeGB --region $Region --app $AppName
    if ($LASTEXITCODE -ne 0) { Write-Warning "assets ボリュームの作成に失敗した可能性があります。手動で確認してください。" }
} else {
    Write-Host "'assets' ボリュームは既に存在します。"
}

# generate ADMIN_TOKEN if not provided
if (-not $AdminToken) {
    Write-Host "ADMIN_TOKEN が指定されていません。ランダムトークンを生成します。"
    $bytes = New-Object 'System.Byte[]' 32
    (New-Object System.Security.Cryptography.RNGCryptoServiceProvider).GetBytes($bytes)
    $AdminToken = [Convert]::ToBase64String($bytes)
}

Write-Host "シークレットを設定します（ADMIN_TOKEN, ENABLE_ADMIN_UPLOADS=false, AUTO_POPULATE_ASSETS=false）"
& flyctl secrets set ADMIN_TOKEN="$AdminToken" ENABLE_ADMIN_UPLOADS='false' AUTO_POPULATE_ASSETS='false' --app $AppName
if ($LASTEXITCODE -ne 0) { Write-Warning "シークレット設定に失敗した可能性があります。手動で設定してください。" }

Write-Host "デプロイを開始します..."
& flyctl deploy --app $AppName
if ($LASTEXITCODE -ne 0) { ExitWithError "デプロイに失敗しました。ログを確認してください。" }

Write-Host "デプロイ成功。ログを確認するには次を実行してください："
Write-Host "  flyctl logs --app $AppName --since 1h"
Write-Host "アプリへのアクセスは Fly の URL を参照してください（例: https://$AppName.fly.dev）。"

Write-Host "注意: このスクリプトは flyctl にログイン済みであることを前提としています。flyctl の対話的ログインが必要な場合は 'flyctl auth login' を先に実行してください。"
