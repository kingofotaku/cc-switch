#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$RepoRoot,
    [string]$BaseRef = 'origin/main',
    [switch]$Json
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
}

function Invoke-Git([string[]]$Arguments) {
    $output = & git -C $RepoRoot @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed: $($output -join [Environment]::NewLine)"
    }
    return @($output)
}

$head = (Invoke-Git @('rev-parse', 'HEAD') | Select-Object -First 1).Trim()
$base = (Invoke-Git @('rev-parse', $BaseRef) | Select-Object -First 1).Trim()
$mergeBase = (Invoke-Git @('merge-base', $BaseRef, 'HEAD') | Select-Object -First 1).Trim()
$status = @(Invoke-Git @('status', '--porcelain'))
$changed = @(Invoke-Git @('diff', '--name-only', "$BaseRef...HEAD"))
$diffCheck = @( & git -C $RepoRoot diff --check 2>&1 )
$diffCheckExit = $LASTEXITCODE

$result = [ordered]@{
    repo_root = (Resolve-Path -LiteralPath $RepoRoot).Path
    base_ref = $BaseRef
    base_commit = $base
    head_commit = $head
    merge_base = $mergeBase
    branch_contains_base = ($mergeBase -eq $base)
    working_tree_clean = ($status.Count -eq 0)
    changed_files = $changed
    diff_check_passed = ($diffCheckExit -eq 0)
}

if ($Json) {
    $result | ConvertTo-Json -Depth 5
}
else {
    Write-Host 'Model-aware source verification'
    $result.GetEnumerator() | ForEach-Object {
        if ($_.Value -is [Array]) {
            Write-Host ("{0}: {1}" -f $_.Key, ($_.Value -join ', '))
        }
        else {
            Write-Host ("{0}: {1}" -f $_.Key, $_.Value)
        }
    }
    if ($diffCheck.Count -gt 0) {
        $diffCheck | ForEach-Object { Write-Host $_ }
    }
}

if ($mergeBase -ne $base -or $diffCheckExit -ne 0) {
    exit 2
}
