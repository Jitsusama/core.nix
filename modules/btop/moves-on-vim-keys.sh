# shellcheck shell=bash
# Presses Vim's keys in btop, as a terminal sends them through tmux, and
# reads which process it says is chosen, from the "1/7" at the foot of its
# process box: j chooses the one below, k goes back up off it, killing
# nothing, Ctrl+d and Ctrl+u move half the list and back, Escape lets go
# of nothing, G chooses the last and gg the first, a g alone waiting for
# its second, and - goes out of the details Return opens. Run as
# `moves-on-vim-keys.sh`, with btop.conf in ./.config/btop. It exits
# non-zero, saying what it saw, when a key didn't go where Vim's goes.
set -eu

# Enough processes that the list runs past the box, so half of it is a
# move of its own, each ended as the script does.
sleepers=()
for _ in $(seq 60); do
  sleep 600 &
  sleepers+=("$!")
done

export TMUX_TMPDIR=$PWD
tmux new-session -d -x 120 -y 40 btop
trap 'tmux kill-server 2>/dev/null || true; kill "${sleepers[@]}" 2>/dev/null || true' EXIT

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

# Prints what btop drew, then what went wrong, last, where a build's log
# shows it, and exits.
fail() {
  echo "btop drew:"
  echo "$pane"
  echo "$1"
  exit 1
}

# Waits up to a minute for btop to say the process chosen is the one
# given: a number, "last", "before-last", or "past N", one further down
# than N and not the last, so a busy machine can't fail it.
expect() {
  for _ in $(seq 600); do
    read_pane
    if [[ -n $chosen ]]; then
      case $1 in
        last) [[ $chosen == "$count" && $chosen != 0 ]] && return 0 ;;
        before-last) [[ $chosen == $((count - 1)) ]] && return 0 ;;
        past) ((chosen > $2 + 1 && chosen < count)) && return 0 ;;
        *) [[ $chosen == "$1" ]] && return 0 ;;
      esac
    fi
    sleep 0.1
  done
  fail "${@: -1}"
}

# Waits up to a minute for btop to draw the details' Elapsed, or, given
# "gone", to draw it no more.
expect_details() {
  for _ in $(seq 600); do
    read_pane
    if [[ $pane == *Elapsed:* ]]; then
      [[ $1 == shown ]] && return 0
    else
      [[ $1 == gone ]] && return 0
    fi
    sleep 0.1
  done
  fail "$2"
}

expect 0 "btop never drew its processes with none chosen"
tmux send-keys j
expect 1 "j didn't choose the first process"
tmux send-keys k
expect 0 "k didn't go back up off the process j chose"
tmux send-keys j
expect 1 "j didn't choose the first process again"
tmux send-keys C-d
expect past 1 "Ctrl+d didn't move half the list down"
tmux send-keys C-u
expect 1 "Ctrl+u didn't move half the list back up"
# Escape only leaves the filter, so the j after it chooses the next. Keys
# go a moment apart, as a hand presses them, since btop drops what comes
# in one read as one key it doesn't know.
tmux send-keys Escape
sleep 0.5
tmux send-keys j
expect 2 "Escape let go of the process chosen, where only the filter's Escape leaves"
# G and gg move a choice to the last and the first, as End and Home do;
# with nothing chosen they only scroll, so one is chosen above.
tmux send-keys G
expect last "G didn't choose the last process"
tmux send-keys g
sleep 0.5
tmux send-keys k
expect before-last "g alone went to the first process, rather than waiting for a second"
# A pair's second comes well inside the language's wait.
tmux send-keys g
sleep 0.1
tmux send-keys g
expect 1 "gg didn't go back to the first process"
tmux send-keys Enter
expect_details shown "Return didn't show the chosen process's details"
tmux send-keys -- -
expect_details gone "- didn't go out of the process's details"
