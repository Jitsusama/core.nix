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
# use his keys (refuses-agents.patch): by the cgroups of the asker and
# then each of its parents up to init, the first a line of the gate names
# deciding, where a chain naming none is his. The gate is the agent's own
# file, SSH_ASKPASS_GATE, read at each ask as the agent reads it at each
# connection, so a gate opening with watch journals what it would refuse
# and asks, as the agent serves; where that file is gone, as on a machine
# without the agent, it's the copy this was built with. So his terminals,
# his editor and ssh-tpm-agent asking for his PIN are asked for, as the
# agent serves them. As the agent does, it refuses for everyone while its
# gate can't be read or parsed, and refuses an asker whose chain can't be
# read to init, a parent gone as it's read or past 32 of them.
#
# Three places the agent serves are refused here too, since a key is used
# by work unattended and a prompt never should be. nix-offload's serving
# units sign in to the builders with keys of their own, which the gate
# never sees, and on 21 November at 22:37 and again at 00:17, copies of
# theirs that lost their pinned host keys put a host-key question on his
# screen for each builder. background.slice holds his daemons, none of
# which asks him through this, and units agents name there. And a cgroup
# read as the root is one seen through a namespace of its own, a fence's,
# since nothing of his runs there. Each refusal goes to the journal at
# warning, saying where the asker ran. Like the gate, this catches an
# agent's mistake: a process set on it can move itself among his slices,
# and stopping that is the fence's work.
own=('/offload\.slice/' '/background\.slice/' '^/$')
refuse() {
  logger -p user.warning -t ssh-askpass -- "refused to ask Joel, as $1" 2> /dev/null || true
  printf '%s\n' "ssh-askpass: refused, as $1" \
    "  only what Joel runs asks him; unattended work uses BatchMode and pinned host keys" >&2
  exit 1
}
gate=${SSH_ASKPASS_GATE:-}
[[ -e $gate || -L $gate ]] || gate=${SSH_ASKPASS_GATE_BUILT:-$gate}
# The gate's lines, parsed as the agent's parseGate does: a word, then a
# pattern after the first space, blank lines and comments skipped.
rules=() verdicts=() watching=0 broken=
text=
if [[ -f $gate && -r $gate ]] && text=$(< "$gate"); then
  n=0
  while IFS= read -r line; do
    n=$((n + 1))
    line=${line#"${line%%[![:space:]]*}"}
    line=${line%"${line##*[![:space:]]}"}
    [[ -z $line || $line == '#'* ]] && continue
    word=${line%% *} pattern=
    [[ $line != *' '* ]] || pattern=${line#* }
    pattern=${pattern#"${pattern%%[![:space:]]*}"}
    if [[ $word == watch && -z $pattern ]]; then
      watching=1
    elif [[ ($word == allow || $word == refuse) && -n $pattern ]]; then
      # Bash's matcher answers 2 for a pattern it can't read, and the
      # condition's own status is the one wanted.
      status=0
      # shellcheck disable=SC2319
      [[ '' =~ $pattern ]] || status=$?
      [[ $status -ne 2 ]] || broken="its line $n isn't a pattern"
      rules+=("$pattern") verdicts+=("$word")
    else
      broken="its line $n is neither allow, refuse nor watch"
    fi
  done <<< "$text"
  [[ -n $broken ]] || [[ " ${verdicts[*]} " == *' refuse '* ]] || broken="it refuses no place"
else
  broken="it can't be read"
fi
[[ -z $broken ]] || refuse "its gate (${gate:-none}) can't be judged by: $broken"
# The asker's chain: its cgroup, then each parent's up to init.
places=() pids=()
pid=$PPID whole=0 lost=
while :; do
  if [[ $pid == 1 ]]; then
    whole=1
    break
  fi
  if [[ ${#places[@]} -eq 32 ]]; then
    lost="has more than 32 parents"
    break
  fi
  if ! cgroups=$(cat "$proc/$pid/cgroup" 2> /dev/null); then
    lost="is gone, or its cgroup can't be read"
    break
  fi
  where=$(sed -n 's/^0:://p' <<< "$cgroups")
  if [[ -z $where ]]; then
    lost="has no unified cgroup"
    break
  fi
  places+=("$where") pids+=("$pid")
  pid=$(sed -n 's/^PPid:[[:space:]]*//p' "$proc/$pid/status" 2> /dev/null || true)
  if ! [[ $pid =~ ^[1-9][0-9]*$ ]]; then
    lost="is gone, or its status can't be read"
    break
  fi
done
# Who the first place the gate names belongs to, said as it reads.
who() {
  if [[ $1 == "$PPID" ]]; then
    echo "$asker (pid $PPID) runs in $2"
  else
    echo "$asker (pid $PPID) was started from $2"
  fi
}
verdict='' at=''
for i in "${!places[@]}"; do
  for rule in "${own[@]}"; do
    [[ ${places[i]} =~ $rule ]] && refuse "$(who "${pids[i]}" "${places[i]}")"
  done
  for j in "${!rules[@]}"; do
    if [[ ${places[i]} =~ ${rules[j]} ]]; then
      verdict=${verdicts[j]} at=$i
      break 2
    fi
  done
done
why=
if [[ $whole != 1 ]]; then
  if [[ ${#pids[@]} -eq 0 ]]; then
    why="$asker (pid $PPID) $lost"
  else
    why="$asker (pid $PPID) has a parent (pid $pid) whose place can't be read: it $lost"
  fi
elif [[ $verdict == refuse ]]; then
  why=$(who "${pids[at]}" "${places[at]}")
fi
if [[ -n $why ]]; then
  [[ $watching == 1 ]] || refuse "$why"
  logger -p user.warning -t ssh-askpass -- \
    "would have refused to ask Joel, as $why, but its gate watches" 2> /dev/null || true
fi
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
