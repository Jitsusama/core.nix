# shellcheck shell=bash
# Whether a checkout's branch was merged as it stands, as the disk guard
# asks a finished source (src/finished.rs): a merged pull request on GitHub
# whose head is the checkout's. More work on top isn't finished, and a
# checkout on no branch never is. The guard gives the checkout in
# DISK_GUARD_REMOTE, DISK_GUARD_BRANCH and DISK_GUARD_HEAD, and takes
# anything but a finished answer, a gh that fails included, as not
# finished.
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
