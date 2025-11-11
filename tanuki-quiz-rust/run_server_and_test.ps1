if (Get-Process -Name tanuki-quiz-rust -ErrorAction SilentlyContinue) { Stop-Process -Name tanuki-quiz-rust -Force }
Start-Process -FilePath ".\target\debug\tanuki-quiz-rust.exe" -WindowStyle Hidden
Start-Sleep -Seconds 1
$uri='http://127.0.0.1:3000/api/generate_quiz'
for ($i=0;$i -lt 15;$i++) {
  try {
    $res = Invoke-RestMethod -Uri $uri -UseBasicParsing -TimeoutSec 2
    $res | ConvertTo-Json -Depth 5
    exit 0
  } catch {
    Start-Sleep -Seconds 1
  }
}
Write-Error 'API did not respond in time'
exit 1
