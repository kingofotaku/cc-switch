#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$RepoRoot,
    [string]$BaseRef,
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

$manifestPath = Join-Path $RepoRoot 'runtime\model-aware-runtime-manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath)) {
    throw "Runtime manifest not found: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace($BaseRef)) {
    $BaseRef = [string]$manifest.base_commit
}
if ([string]::IsNullOrWhiteSpace($BaseRef)) {
    throw "BaseRef was not provided and the runtime manifest has no base_commit: $manifestPath"
}

$head = (Invoke-Git @('rev-parse', 'HEAD') | Select-Object -First 1).Trim()
$base = (Invoke-Git @('rev-parse', "$BaseRef^{commit}") | Select-Object -First 1).Trim()
$mergeBase = (Invoke-Git @('merge-base', $base, 'HEAD') | Select-Object -First 1).Trim()
$status = @(Invoke-Git @('status', '--porcelain'))
$changed = @(Invoke-Git @('diff', '--name-only', "$base...HEAD"))
$diffCheck = @( & git -C $RepoRoot diff --check 2>$null )
$diffCheckExit = $LASTEXITCODE
$package = Get-Content -LiteralPath (Join-Path $RepoRoot 'package.json') -Raw | ConvertFrom-Json
$manifestBaseMatches = ([string]$manifest.base_commit -eq $base)
$manifestVersionMatches = ([string]$manifest.cc_switch_version -eq [string]$package.version)
$workingTreeClean = ($status.Count -eq 0)

$result = [ordered]@{
    repo_root = (Resolve-Path -LiteralPath $RepoRoot).Path
    base_ref = $BaseRef
    base_commit = $base
    head_commit = $head
    merge_base = $mergeBase
    branch_contains_base = ($mergeBase -eq $base)
    manifest_base_matches = $manifestBaseMatches
    manifest_version_matches = $manifestVersionMatches
    working_tree_clean = $workingTreeClean
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

if (
    $mergeBase -ne $base -or
    -not $manifestBaseMatches -or
    -not $manifestVersionMatches -or
    -not $workingTreeClean -or
    $diffCheckExit -ne 0
) {
    exit 2
}
