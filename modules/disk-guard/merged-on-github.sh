# shellcheck shell=bash
# Whether a checkout's branch was merged as it stands, as the disk guard
# asks a finished source (src/finished.rs): a merged pull request on GitHub
# whose head is the checkout's. More work on top isn't finished, and a
# checkout on no branch never is. The guard gives the checkout in
# DISK_GUARD_REMOTE, DISK_GUARD_BRANCH and DISK_GUARD_HEAD, and takes
# anything but a finished answer, a gh that fails included, as not
# finished. Its help takes the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
disk-guard-merged-on-github  say if a branch merged as it is

  The disk guard asks it of a checkout of GitHub's, given
  in DISK_GUARD_REMOTE, DISK_GUARD_BRANCH and
  DISK_GUARD_HEAD. It answers {"finished": true} when a
  merged pull request's head is the checkout's.

EXAMPLES
  DISK_GUARD_REMOTE=https://github.com/owner/repo \
    DISK_GUARD_BRANCH=tidy DISK_GUARD_HEAD=4f0c2e1 \
    disk-guard-merged-on-github

EXIT
  0   answered
  else gh's own, which the guard reads as not finished
EOF
  exit 0
  ;;
*) ;;
esac
repository=$(printf '%s' "$DISK_GUARD_REMOTE" |
  sed -E 's#^(https://github\.com/|git@github\.com:|ssh://git@github\.com/)##; s#\.git$##')
if [ -z "$DISK_GUARD_BRANCH" ]; then
  echo '{"finished": false}'
  exit
fi
heads=$(gh pr list --repo "$repository" --head "$DISK_GUARD_BRANCH" \
  --state merged --json headRefOid --jq '.[].headRefOid')
if printf '%s\n' "$heads" | grep -qxF "$DISK_GUARD_HEAD"; then
  echo '{"finished": true}'
else
  echo '{"finished": false}'
fi
