#compdef eero
# zsh's completion for eero, which `eero completion zsh` writes. It
# follows eero's tree as its help does: eero says what may come after the
# words typed (`eero names next <words>`), a collection's names read from
# the record the daemon keeps, so what's offered is what's on the home
# network now, and each subcommand after.

_eero() {
  local -a typed next
  # What's typed before the word being completed, its flags left out,
  # since a view takes them anywhere.
  typed=(${(Q)${words[2,CURRENT-1]}:#-*})
  if [[ $PREFIX == -* ]]; then
    compadd -- --json --watch --help
    return
  fi
  next=(${(f)"$(_call_program next eero names next ${(q)typed} 2>/dev/null)"})
  _describe -t next next next
}

_eero "$@"
