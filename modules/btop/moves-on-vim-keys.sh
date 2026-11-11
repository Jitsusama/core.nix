# shellcheck shell=bash
# Presses Vim's keys in btop, as a terminal sends them through tmux, and
# reads which process it says is chosen, from the "1/7" at the foot of its
# process box: j chooses the one below, k goes back up off it, killing
# nothing, G chooses the last and g the first. Run as `moves-on-vim-keys.sh`, with
# btop.conf in ./.config/btop. It exits non-zero, saying what it saw, when
# a key didn't go where Vim's goes.
set -eu

export TMUX_TMPDIR=$PWD
tmux new-session -d -x 120 -y 40 btop
trap 'tmux kill-server 2>/dev/null || true' EXIT

drawn() {
  tmux capture-pane -p
}

# The chosen process and how many there are, as the process box's foot
# says them, 0 chosen being none.
chosen() {
  drawn | grep -o -E '[0-9]+/[0-9]+└╯' | tail -n 1 | sed 's/└╯//'
}

# Waits up to a minute for btop to say the process chosen is the one
# given, as "1" or "last", so a busy machine can't fail it.
waits() {
  for _ in $(seq 600); do
    said=$(chosen)
    if [ -n "$said" ]; then
      if [ "$1" = last ]; then
        [ "${said%/*}" = "${said#*/}" ] && [ "${said%/*}" != 0 ] && return 0
      else
        [ "${said%/*}" = "$1" ] && return 0
      fi
    fi
    sleep 0.1
  done
  return 1
}

# What btop drew, then what went wrong, last, where a build's log shows it.
saw() {
  echo "btop drew:"
  drawn
  echo "$1"
  exit 1
}

waits 0 || saw "btop never drew its processes with none chosen"
tmux send-keys j
waits 1 || saw "j didn't choose the first process"
tmux send-keys k
waits 0 || saw "k didn't go back up off the process j chose"
# G and g move a choice to the last and the first, as End and Home do; with
# nothing chosen they only scroll, so one is chosen first.
tmux send-keys j
waits 1 || saw "j didn't choose the first process again"
tmux send-keys G
waits last || saw "G didn't choose the last process"
tmux send-keys g
waits 1 || saw "g didn't go back to the first process"
