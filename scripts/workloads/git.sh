#!/bin/sh
set -e
export PATH=/usr/bin:/bin
export HOME=/tmp GIT_PAGER=cat PAGER=cat GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_DATE='2026-01-01T00:00:00 +0000'
export GIT_COMMITTER_DATE='2026-01-01T00:00:00 +0000'
rm -rf /tmp/gitrepo /tmp/gitclone
mkdir -p /tmp/gitrepo && cd /tmp/gitrepo
git --version
git -c init.defaultBranch=main init -q
git config user.email test@example.com
git config user.name Test
echo hello > a.txt
git add a.txt
git commit -qm first
git log --oneline | head -1
echo change >> a.txt
git diff --stat
git status --porcelain
git clone -q /tmp/gitrepo /tmp/gitclone
cd /tmp/gitclone
git log --oneline | head -1
git fsck --no-progress
echo GIT_OK
