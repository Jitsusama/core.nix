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

# A prompt is never --help, so that answers its help, and anything but one
# prompt is refused, so a person trying it never puts a prompt on the
# screen. Its help and refusals take docs/commands.md's shape, by hand.
case ${1-} in
  -h | --help)
    cat << 'EOF'
ssh-askpass  asks for a secret or a yes through the shell

EXAMPLES
  ssh-askpass "PIN for the key:"   what ssh runs to ask

EXIT
  0   answered, on standard output
  1   refused, or no shell answered
  64  asked wrongly
EOF
    exit 0
    ;;
  *) ;;
esac
if [ "$#" -ne 1 ]; then
  printf '%s\n' "ssh-askpass needs one prompt" "  try  ssh-askpass --help" >&2
  exit 64
fi

kind=${SSH_ASKPASS_PROMPT:-secret}
# qs finds the shell by the display it draws on, and a program started
# before the desktop, as linux-id's service is at sign-in, was never
# told it, so it's taken from the session the account's systemd holds.
# With no systemd to ask, there's no shell to find either, and qs says so.
if [[ -z ${WAYLAND_DISPLAY:-} ]] && environment=$(systemctl --user show-environment 2> /dev/null); then
  display=$(sed -n 's/^WAYLAND_DISPLAY=//p' <<< "$environment")
  [[ -z $display ]] || export WAYLAND_DISPLAY=$display
fi
# Where the kernel keeps each process's words and cgroup; the askpass's
# tests point it at a folder of their own.
proc=/proc
# Who asked: the program that ran this, or the script an interpreter
# runs, named without its store path or a wrapper's dots.
argv=()
mapfile -d "" -t argv < "$proc/$PPID/cmdline" || true
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
# Only what Joel runs may put a prompt on his screen, and agents' work is
# refused without asking, judged as ssh-tpm-agent's gate judges who may
# use his keys: by the cgroups of the asker and then each of its parents,
# against the gate's refuse lines (modules/ssh-tpm-agent/gate.conf, which
# SSH_ASKPASS_GATE names), where a chain naming none is his. So his
# terminals, his editor and ssh-tpm-agent asking for his PIN are asked
# for, as the agent serves them. nix-offload's serving units are refused
# too: they sign in to the builders with keys of their own, which the gate
# never sees, and on 21 November at 22:37 and again at 00:17, copies of
# theirs that lost their pinned host keys put a host-key question on his
# screen for each builder. A gate that can't be read, or a caller whose
# cgroup can't be, refuses. Each refusal goes to the journal saying where
# the asker ran. Like the gate, this catches an agent's mistake: a
# process set on it can move itself among his slices, and stopping that
# is the fence's work.
refusing=('/offload\.slice/')
rules=$(sed -n 's/^refuse //p' "${SSH_ASKPASS_GATE:-}" 2> /dev/null || true)
[[ -z $rules ]] || mapfile -t -O 1 refusing <<< "$rules"
refuse() {
  logger -t ssh-askpass -- "refused to ask Joel, as $1" 2> /dev/null || true
  printf '%s\n' "ssh-askpass: refused, as $1" \
    "  only what Joel runs asks him; unattended work uses BatchMode and pinned host keys" >&2
  exit 1
}
[[ -n $rules ]] || refuse "its gate (${SSH_ASKPASS_GATE:-none}) can't be read"
place=$(sed -n 's/^0:://p' "$proc/$PPID/cgroup" 2> /dev/null || true)
[[ -n $place ]] || refuse "$asker (pid $PPID) is gone, or its cgroup can't be read"
pid=$PPID where=$place
# The chain ends at the kernel's own parent, pid 0, or at a parent gone,
# and is cut off after 64, which no real one reaches.
for _ in $(seq 64); do
  for rule in "${refusing[@]}"; do
    if [[ $where =~ $rule ]]; then
      [[ $pid == "$PPID" ]] || refuse "$asker (pid $PPID) was started from $where"
      refuse "$asker (pid $PPID) runs in $where"
    fi
  done
  pid=$(sed -n 's/^PPid:[[:space:]]*//p' "$proc/$pid/status" 2> /dev/null || true)
  [[ $pid =~ ^[1-9][0-9]*$ ]] || break
  where=$(sed -n 's/^0:://p' "$proc/$pid/cgroup" 2> /dev/null || true)
  [[ -n $where ]] || break
done
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
  echo "ssh-askpass: the desktop isn't there to ask" >&2
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
