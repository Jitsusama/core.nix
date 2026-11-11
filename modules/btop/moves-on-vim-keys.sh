# shellcheck shell=bash
# Presses Vim's keys in btop, as a terminal sends them through tmux, and
# reads which process it says is chosen, from the "1/7" at the foot of its
# process box: j chooses the one below, k goes back up off it, killing
# nothing, G chooses the last and g the first. Run as
# `moves-on-vim-keys.sh`, with btop.conf in ./.config/btop. It exits
# non-zero, saying what it saw, when a key didn't go where Vim's goes.
set -eu

export TMUX_TMPDIR=$PWD
tmux new-session -d -x 120 -y 40 btop
trap 'tmux kill-server 2>/dev/null || true' EXIT

# What btop drew last, in `pane`, and the chosen process and how many there
# are, as the process box's foot says them, in `chosen` and `count`, 0
# chosen being none, both empty before it draws its processes.
read_pane() {
  pane=$(tmux capture-pane -p)
  chosen=""
  count=""
  if [[ $pane =~ ([0-9]+)/([0-9]+)└╯ ]]; then
    chosen=${BASH_REMATCH[1]}
    count=${BASH_REMATCH[2]}
  fi
}

# Waits up to a minute for btop to say the process chosen is the one
# given, as "1" or "last", so a busy machine can't fail it; otherwise it
# prints what btop drew, then what went wrong, last, where a build's log
# shows it, and exits.
expect() {
  for _ in $(seq 600); do
    read_pane
    if [[ -n $chosen ]]; then
      if [[ $1 == last ]]; then
        [[ $chosen == "$count" && $chosen != 0 ]] && return 0
      else
        [[ $chosen == "$1" ]] && return 0
      fi
    fi
    sleep 0.1
  done
  echo "btop drew:"
  echo "$pane"
  echo "$2"
  exit 1
}

expect 0 "btop never drew its processes with none chosen"
tmux send-keys j
expect 1 "j didn't choose the first process"
tmux send-keys k
expect 0 "k didn't go back up off the process j chose"
# G and g move a choice to the last and the first, as End and Home do; with
# nothing chosen they only scroll, so one is chosen first.
tmux send-keys j
expect 1 "j didn't choose the first process again"
tmux send-keys G
expect last "G didn't choose the last process"
tmux send-keys g
expect 1 "g didn't go back to the first process"
