# Single verification command of the project. Every check that must stay true is attached here.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File check.ps1
# Exit code: 0 when every step is green, 1 otherwise. Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$results = New-Object System.Collections.ArrayList

function Invoke-Step([string]$name, [scriptblock]$action) {
    Write-Host ""
    Write-Host "== $name" -ForegroundColor Cyan
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $global:LASTEXITCODE = 0
    & $action
    $ok = ($LASTEXITCODE -eq 0)
    $watch.Stop()
    [void]$results.Add([pscustomobject]@{ Name = $name; Ok = $ok; Seconds = [math]::Round($watch.Elapsed.TotalSeconds, 1) })
}

Invoke-Step 'format' { cargo fmt --all -- --check }
Invoke-Step 'clippy (workspace, no warning)' { cargo clippy --workspace --all-targets -- -D warnings }
Invoke-Step 'tests (workspace)' { cargo test --workspace --quiet }

Write-Host ""
$failed = @($results | Where-Object { -not $_.Ok })
foreach ($r in $results) {
    $color = 'Green'; $mark = 'OK  '
    if (-not $r.Ok) { $color = 'Red'; $mark = 'FAIL' }
    Write-Host ("{0} {1,-40} {2,7}s" -f $mark, $r.Name, $r.Seconds) -ForegroundColor $color
}
if ($failed.Count -gt 0) {
    Write-Host "VERDICT: RED ($($failed.Count) step(s) failed)" -ForegroundColor Red
    exit 1
}
Write-Host 'VERDICT: GREEN' -ForegroundColor Green
exit 0
