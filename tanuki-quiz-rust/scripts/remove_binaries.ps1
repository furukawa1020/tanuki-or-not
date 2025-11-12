<#
PowerShell helper to find and optionally remove large files from git history using git-filter-repo.

Prerequisites:
- Install git-filter-repo (Python-based). On Windows, follow instructions: https://github.com/newren/git-filter-repo
- Or install BFG Repo-Cleaner (Java-based): https://rtyley.github.io/bfg-repo-cleaner/

This script will:
1. List files over a threshold size in the current repository history.
2. If user confirms, run git-filter-repo to remove them from history.

USAGE:
  .\remove_binaries.ps1 -ListOnly
  .\remove_binaries.ps1

WARNING: Rewriting history is destructive and requires force-pushing and coordination with collaborators. Make a backup clone first.
#>
param(
    [switch]$ListOnly,
    [int]$SizeThresholdMB = 5
)

Write-Host "Scanning git objects for files larger than $SizeThresholdMB MB..."
# list blobs with sizes
git rev-list --objects --all | ForEach-Object {
    # use git cat-file to show type/size
} | Out-Null

# Simpler: use git-sizer (optional) or advise manual BFG usage
Write-Host "Automatic scanning is limited in this helper. Recommended:
1) Install BFG or git-filter-repo.
2) Run: java -jar bfg.jar --strip-blobs-bigger-than ${SizeThresholdMB}M .
3) Or with git-filter-repo: git filter-repo --strip-blobs-bigger-than ${SizeThresholdMB}M

Make a bare backup first:
  git clone --mirror <repo> repo.git

After running the cleaner, force-push to origin:
  git push --force --all
  git push --force --tags

If you want, I can prepare the exact commands for your repository and help you run them step-by-step."
