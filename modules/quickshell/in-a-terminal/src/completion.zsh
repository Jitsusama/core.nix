#compdef launcher
# zsh's completion for launcher, which `launcher completion zsh` writes:
# its commands, and after `show` the rows the place the steps typed so far
# reach holds, which launcher asks the running shell for (`launcher names
# <step ...>`), so what's offered is what the launcher holds now.

_launcher() {
  local -a typed commands named
  # What's typed before the word being completed, unquoted, but the flags,
  # which show takes anywhere.
  typed=(${(Q)${words[2,CURRENT-1]}:#-*})
  if (( ${#typed} == 0 )); then
    commands=('show:any place, row or drawer the launcher holds')
    _describe -t commands command commands
    return
  fi
  [[ $typed[1] == show ]] || return 1
  if [[ $PREFIX == -* ]]; then
    compadd -- --drawers --json
    return
  fi
  named=(${(f)"$(_call_program names launcher names ${(q)typed[2,-1]} 2>/dev/null)"})
  _describe -t names row named
}

_launcher "$@"
