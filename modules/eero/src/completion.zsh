#compdef eero
# zsh's completion for eero, which `eero completion zsh` writes: its views
# and commands, and the home network's nodes, devices and profiles, which
# eero names from the record the daemon keeps (`eero names <kind>`), so
# what's offered is what's on the home network now.

# Offers what eero names of a kind, each with what it is.
_eero_named() {
  local -a named
  named=(${(f)"$(_call_program $1 eero names $1 2>/dev/null)"})
  _describe -t $1 $2 named
}

_eero() {
  local -a typed commands
  local watching=0
  # What's typed before the word being completed, but --json, which a
  # view takes anywhere.
  typed=(${${words[2,CURRENT-1]}:#--json})
  if [[ $typed[1] == watch ]]; then
    watching=1
    typed=(${typed[2,-1]})
  fi
  if (( ${#typed} == 0 )); then
    if (( ! watching )); then
      commands=(
        'watch:a view, drawn again as the home network changes'
        "speedtest:start the eero's own speed test"
        'sqm:Optimize for Conferencing and Gaming, on or off'
        'sign-in:sign in once, typing the code eero sends'
      )
      _describe -t commands command commands
    fi
    _eero_named views view
    return
  fi
  case $typed[1] in
    node) (( ${#typed} == 1 )) && _eero_named nodes node ;;
    device) (( ${#typed} == 1 )) && _eero_named devices device ;;
    profile) (( ${#typed} == 1 )) && _eero_named profiles profile ;;
    sqm) (( ${#typed} == 1 && ! watching )) && compadd on off ;;
  esac
  if [[ $typed[1] != (sqm|speedtest|sign-in) && $PREFIX == -* ]]; then
    compadd -- --json
  fi
}

_eero "$@"
