#!/usr/bin/env zsh

# Declare all local variables.
local opt threads opts folder base title
local border=true
local folders=(~/src ~/.config)

# Parse command line options.
while getopts "n" opt; do
    case $opt in
        n) border=false ;;
    esac
done
shift $((OPTIND-1))

# Use all available CPU cores for optimal I/O performance.
if (( $+commands[nproc] )); then
    threads=$(nproc)
else
    threads=$(sysctl -n hw.ncpu)
fi

# Build fzf command with options.
if ! $border; then
    opts=(--no-border --no-scrollbar --margin=1,3,0,3)
else
    opts=()
fi

# Find git repositories and let user select one.
folder=$(fd -H -t d --max-depth 5 -j "$threads" '^\.git$' ${folders[@]} | \
    sed 's|/\.git/$||' | \
    sort | \
    fzf ${opts[@]} --query="$*") || return 0

# Convert the folder name into a title case tab name.
base=${folder:t}
title=${base//[-_]/ }
title=${(C)title}

# Check if there are any active (non-EXITED) sessions.
local active_sessions=$(zellij list-sessions 2>/dev/null | \
    grep -v "EXITED" | \
    grep -c "current\|RUNNING" || true)

# Launch the Zellij layout.
if [[ $active_sessions -gt 0 ]]; then
    zellij action new-tab --name "$title" --cwd "$folder" --layout zdev
else
    local session_name="zdev-$(date -u +%Y%m%d%H%M%S)Z-$$"
    cd "$folder" && exec zellij --session "$session_name" --new-session-with-layout zdev
fi
