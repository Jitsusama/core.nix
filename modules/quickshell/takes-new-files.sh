# shellcheck shell=bash
# Brings the shell onto the files its unit names now, once the launcher is
# idle. A switch leaves the running shell alone (home.nix's keep-old), since
# a restart inside the switch came wherever the switch's turn fell: at 18:08
# on 13 November one landed 60 s after the launcher closed, as Joel opened
# it. This then hands the shell over once the launcher and Super+S's talk line
# have been shut 30 s, or after an hour of them in use at their next close,
# never shutting one under Joel's hands, so a switch that changes the shell
# is on screen the same night. Its words are the list of the shell's
# files the unit names now and, for its check, how long to wait at most, in
# seconds. The shell's start links the list it started from in
# $XDG_RUNTIME_DIR/quickshell-files, so one already on its files is left be.
files=$1
waits_at_most=${2:-3600}

# The unit's reload hands it over to a new shell, the old one answering
# until the new one is ready (shell-hands-over.sh), where a restart left no
# launcher, bar or lock client for about a second. A handover that refuses,
# as when the new shell won't load, leaves the old one, so it restarts.
take() {
  systemctl --user reload quickshell.service && exit 0
  echo "the handover refused, so the shell restarts onto its new files"
  exec systemctl --user restart quickshell.service
}
runtime=${XDG_RUNTIME_DIR:?}
marks=("${runtime}/quickshell-launcher" "${runtime}/quickshell-talk-line")

linked=$(readlink "${runtime}/quickshell-files" 2>/dev/null || true)
if [[ ${linked} == "${files}" ]]; then
  exit 0
fi
if ! systemctl --user --quiet is-active quickshell.service; then
  echo "the shell isn't running, so it takes its new files as it starts"
  exit 0
fi

# The shell writes "open" or "closed" in each mark only as it changes, so a
# mark's time dates the close, and someone who just pressed Return in it is
# still at it. The talk line's mark is there only once the line writes it.
up() {
  local mark
  for mark in "${marks[@]}"; do
    grep -qx open "${mark}" 2>/dev/null && return 0
  done
  return 1
}
in_use() {
  up && return 0
  local mark written now
  now=$(date +%s)
  for mark in "${marks[@]}"; do
    written=$(stat -c %Y "${mark}" 2>/dev/null) || continue
    ((now - written < 30)) && return 0
  done
  return 1
}

started=$(date +%s)
while in_use; do
  now=$(date +%s)
  if ((now - started >= waits_at_most)); then
    while up; do
      sleep 1
    done
    echo "the launcher stayed in use $((waits_at_most / 60)) min, so the shell takes its new" \
      "files as it shuts"
    take
  fi
  sleep 1
done
now=$(date +%s)
echo "the shell takes its new files, the launcher shut 30 s, $((now - started)) s after the switch"
take
