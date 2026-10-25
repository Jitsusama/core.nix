# shellcheck shell=bash
# OpenSSH's askpass, asked through the shell (Askpass.qml): the prompt is
# the only argument, SSH_ASKPASS_PROMPT says whether it's a secret, a yes
# ("confirm") or a notice that waits on a touch ("none"), the answer goes
# to standard output, and a refusal exits nonzero. The shell answers into
# a folder made for it, watched before asking so the answer can't come
# unseen. A notice lasts until the program stops the askpass, and anything
# that ends it early takes the prompt back down. The shell holds an ask only
# in memory, so while one is up the askpass checks every few seconds that
# the shell still has it: a shell that restarted is asked again, and once
# no shell has answered for 20 seconds the askpass refuses. Otherwise the
# program would wait on it forever, and ssh-tpm-agent every key with it.

kind=${SSH_ASKPASS_PROMPT:-secret}
# qs finds the shell by the display it draws on, and a program started
# before the desktop, as linux-id's service is at sign-in, was never
# told it, so it's taken from the session the account's systemd holds.
# With no systemd to ask, there's no shell to find either, and qs says so.
if [[ -z ${WAYLAND_DISPLAY:-} ]] && environment=$(systemctl --user show-environment 2> /dev/null); then
  display=$(sed -n 's/^WAYLAND_DISPLAY=//p' <<< "$environment")
  [[ -z $display ]] || export WAYLAND_DISPLAY=$display
fi
# Who asked: the program that ran this, or the script an interpreter
# runs, named without its store path or a wrapper's dots.
argv=()
mapfile -d "" -t argv < "/proc/$PPID/cmdline" || true
asker=${argv[0]:-}
asker=${asker##*/}
case $asker in
  bash | sh | dash | zsh | python* | perl)
    script=${argv[1]:-}
    [[ $script == -* || $script == "" ]] || asker=${script##*/}
    ;;
esac
[[ $asker =~ ^[0-9a-z]{32}-(.+)$ ]] && asker=${BASH_REMATCH[1]}
[[ $asker =~ ^\.(.+)-wrapped$ ]] && asker=${BASH_REMATCH[1]}
reply=$(mktemp -d -p "$XDG_RUNTIME_DIR" askpass.XXXXXXXX)
exec 3< <(exec inotifywait -m -e moved_to,close_write --format %f "$reply" 2>&1)
watcher=$!
settled() { [ -e "$reply/answer" ] || [ -e "$reply/refused" ]; }
finish() {
  settled || qs ipc call askpass withdraw "$reply" > /dev/null 2>&1 || true
  kill "$watcher" 2> /dev/null || true
  rm -rf "$reply"
}
trap finish EXIT
trap 'exit 1' TERM INT HUP
while read -r line <&3 && [ "$line" != "Watches established." ]; do :; done

ask() { qs ipc call askpass ask "$kind" "$asker" "$1" "$reply" > /dev/null; }
if ! ask "$1"; then
  echo "ssh-askpass: the shell isn't there to ask" >&2
  exit 1
fi
every=3 unanswered=0
until settled; do
  status=0
  read -r -t "$every" _ <&3 || status=$?
  # Beyond 128 is the wait ending; anything else is the watcher gone.
  [ "$status" -eq 0 ] && continue
  [ "$status" -gt 128 ] || exit 1
  if waiting=$(qs ipc call askpass waiting "$reply" 2> /dev/null); then
    unanswered=0
    # Answered just now, or forgotten by a shell that restarted.
    [ "$waiting" = true ] || settled || ask "$1" || true
  else
    unanswered=$((unanswered + every))
    if [ "$unanswered" -ge 20 ]; then
      echo "ssh-askpass: the shell went away without answering" >&2
      exit 1
    fi
  fi
done
[ -e "$reply/answer" ] || exit 1
[ "$kind" = confirm ] || cat "$reply/answer"
