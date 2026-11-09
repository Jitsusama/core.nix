#compdef passwords
# zsh's completion for passwords, which `passwords completion zsh` writes:
# its commands, each entry by the title that finds it, an entry's details
# by their labels, and the password managers, which passwords names from
# what its daemon holds (`passwords names <kind>`). It asks no password
# manager anything, so a Tab never makes one, as 1Password does, ask Joel
# to let it in; an entry is offered once a list or a search has read it,
# and its details once it's been read in full.

# Offers what passwords names of a kind, each with what it is.
_passwords_named() {
  local tag=$1 what=$2
  shift 2
  local -a named
  named=(${(f)"$(_call_program $tag passwords names ${(q)@} 2>/dev/null)"})
  _describe -t $tag $what named
}

_passwords() {
  local -a typed commands
  # What's typed before the word being completed, but --json, which any
  # command takes anywhere, each unquoted as passwords will read it.
  typed=(${(Q)${words[2,CURRENT-1]:#--json}})
  if (( ${#typed} == 0 )); then
    commands=(
      'find:the entries some words find'
      'list:every entry'
      "details:an entry's details, with no hidden detail's value"
      "show:a detail's value, its secret included"
      "copy:a detail's value onto the clipboard, cleared after 90 s"
      "code:an entry's one-time code now"
      'let-in:ask a password manager to let passwords in'
      'health:how each password manager stands'
      'audit:every password checked for reuse and breaches'
      "keys:the keys 1Password's SSH agent holds"
      'help:what each command does, and the exit statuses'
    )
    _describe -t commands command commands
    return
  fi
  case $typed[1] in
    details|code) (( ${#typed} == 1 )) && _passwords_named entries entry entries ;;
    show|copy)
      case ${#typed} in
        1) _passwords_named entries entry entries ;;
        2) _passwords_named details detail details $typed[2] ;;
      esac
      ;;
    let-in) (( ${#typed} == 1 )) && _passwords_named managers 'password manager' managers ;;
  esac
  if [[ $typed[1] != (let-in|help) && $PREFIX == -* ]]; then
    compadd -- --json
  fi
}

_passwords "$@"
