#compdef launcher
# zsh's completion for launcher, which `launcher completion zsh` writes:
# its commands, and after `show` or `do` the rows the place the steps typed
# so far reach holds, which launcher asks the running shell for (`launcher names
# <step ...>`), so what's offered is what the launcher holds now.

_launcher() {
  local -a typed commands named
  # What's typed before the word being completed, unquoted, but the flags,
  # which show and do take anywhere.
  typed=(${(Q)${words[2,CURRENT-1]}:#-*})
  if (( ${#typed} == 0 )); then
    # Each command and what it does, from words.json, as launcher writes it.
    commands=(@commands@)
    _describe -t commands command commands
    return
  fi
  # After do's --, what's typed is an act's own words.
  (( ${words[(I)--]} > 1 && ${words[(I)--]} < CURRENT )) && return 1
  case $typed[1] in
    show) [[ $PREFIX == -* ]] && { compadd -- --drawers --json; return; } ;;
    do) [[ $PREFIX == -* ]] && { compadd -- --yes --; return; } ;;
    *) return 1 ;;
  esac
  named=(${(f)"$(_call_program names launcher names ${(q)typed[2,-1]} 2>/dev/null)"})
  _describe -t names row named
}

_launcher "$@"
