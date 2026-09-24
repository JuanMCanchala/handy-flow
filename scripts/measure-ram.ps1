<#
.SYNOPSIS
    Measures the memory footprint of a running Voxa/Handy process tree.

.DESCRIPTION
    Finds every process whose image name matches the given executable (the
    main process plus any WebView2 helper processes it spawns:
    msedgewebview2.exe, WebView2 network/GPU/renderer helpers, etc.) and
    reports per-process and total Working Set + Private Bytes.

    Run this manually against a running release build in each state called
    for by docs/perf.md ("window open", "hidden to tray", "after one
    dictation", "60s after model unload"). It does not launch or drive the
    app itself — that part is manual/interactive because it requires a real
    keypress + microphone input.

.PARAMETER ExeName
    Base name (no .exe) of the main process to search for. Defaults to
    "handy", the binary name from src-tauri/Cargo.toml ([package] name and
    the `handy` bin target). WebView2 helper processes are matched by their
    `--webview-exe-name=<ExeName>.exe` command-line argument, which
    correctly excludes msedgewebview2 helpers spawned by other apps (e.g.
    Windows Search/Widgets) sharing the same shared WebView2 runtime.

.PARAMETER Label
    Free-text label for this measurement, printed in the output and CSV so
    a session's samples can be told apart (e.g. "window-open",
    "hidden-to-tray", "after-dictation", "60s-after-unload").

.PARAMETER CsvPath
    Optional path to append a CSV row to, for building the before/after
    table in docs/perf.md.

.EXAMPLE
    .\scripts\measure-ram.ps1 -Label "window-open"

.EXAMPLE
    .\scripts\measure-ram.ps1 -Label "hidden-to-tray" -CsvPath docs\perf-samples.csv
#>
param(
    [string]$ExeName = "handy",
    [string]$Label = "unlabeled",
    [string]$CsvPath
)

$ErrorActionPreference = "Stop"

# msedgewebview2.exe is shared by every WebView2 app on the machine (Windows
# Search/Widgets' SearchHost.exe included), so matching by name alone
# over-counts on a normal desktop. Every WebView2 helper's command line
# carries --webview-exe-name=<host exe>, so filter on that instead of trying
# to walk the process tree (some helpers are children of other
# msedgewebview2 instances, not of the host exe directly).
$mainProcs = @(Get-Process -Name $ExeName -ErrorAction SilentlyContinue)

if (-not $mainProcs) {
    Write-Warning "No process found matching: $ExeName"
    exit 1
}

$webviewHelpers = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
    Where-Object { $_.CommandLine -match "--webview-exe-name=$([regex]::Escape($ExeName))\.exe" })

$procs = @($mainProcs) + @($webviewHelpers | ForEach-Object {
    Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
})
$procs = $procs | Where-Object { $_ }

if (-not $procs) {
    Write-Warning "No processes found matching: $ExeName (or its WebView2 helpers)"
    exit 1
}

$rows = foreach ($p in $procs) {
    [PSCustomObject]@{
        Pid           = $p.Id
        Name          = $p.ProcessName
        WorkingSetMB  = [math]::Round($p.WorkingSet64 / 1MB, 1)
        PrivateMB     = [math]::Round($p.PrivateMemorySize64 / 1MB, 1)
    }
}

$rows = $rows | Sort-Object -Property Name, Pid
$totalWorkingSetMB = [math]::Round(($rows | Measure-Object -Property WorkingSetMB -Sum).Sum, 1)
$totalPrivateMB = [math]::Round(($rows | Measure-Object -Property PrivateMB -Sum).Sum, 1)
$processCount = $rows.Count

Write-Host ""
Write-Host "=== $Label ===" -ForegroundColor Cyan
$rows | Format-Table -AutoSize
Write-Host ("Total: {0} processes, {1} MB working set, {2} MB private" -f `
    $processCount, $totalWorkingSetMB, $totalPrivateMB) -ForegroundColor Green

if ($CsvPath) {
    $exists = Test-Path $CsvPath
    $summary = [PSCustomObject]@{
        Timestamp        = (Get-Date).ToString("s")
        Label            = $Label
        ProcessCount     = $processCount
        TotalWorkingSetMB = $totalWorkingSetMB
        TotalPrivateMB   = $totalPrivateMB
    }
    $summary | Export-Csv -Path $CsvPath -NoTypeInformation -Append:$exists
    Write-Host "Appended summary row to $CsvPath"
}
