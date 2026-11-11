#compdef passwords
# zsh's completion for passwords, which `passwords completion zsh` writes:
# its commands, and --json. No command reaches an entry, since the daemon
# answers the launcher alone, so nothing here can name one.

_passwords() {
  local -a typed commands
  # What's typed before the word being completed, but --json, which any
  # command takes anywhere.
  typed=(${(Q)${words[2,CURRENT-1]:#--json}})
  if (( ${#typed} == 0 )); then
    commands=(
      "keys:the keys 1Password's SSH agent holds"
      'help:what each command does, and the exit statuses'
    )
    _describe -t commands command commands
    return
  fi
  if [[ $typed[1] == keys && $PREFIX == -* ]]; then
    compadd -- --json
  fi
}

_passwords "$@"
