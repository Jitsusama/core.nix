# Completes passwords commands in a real zsh line editor, through zpty,
# with the completion `passwords completion zsh` writes and the real
# passwords, asking the daemon the check started, whose stand-in manager
# logs each ask to $ASKED. It fails unless each start offers what it
# should, and unless no Tab asked the manager anything. A completion here
# prints each match, a bar and how it's shown, between two NULs, and then
# the shell exits, so it's read to the end. Only the matches are held to,
# since how each is described is passwords' to change.
completion=$1
here=$PWD
mkdir -p dot
cp $completion _passwords
# The PATH is set again here, since a machine's /etc/zshenv, which zsh
# reads whatever it's told, may set its own.
cat > dot/.zshrc <<EOF
PROMPT=''
PATH=$PATH
fpath=($here \$fpath)
autoload -Uz compinit
compinit -u -D
zstyle ':completion:*' list-grouped false
zstyle ':completion:*' insert-tab false
EOF
cat >> dot/.zshrc <<'EOF'
compadd() {
  if [[ ${@[1,(i)(-|--)]} == *-(O|A|D)\ * ]]; then
    builtin compadd "$@"
    return $?
  fi
  typeset -a __hits
  builtin compadd -A __hits "$@"
  [[ -n $__hits ]] || return
  print -rl -- $__hits
}
_message() { print -r -- "(said) ${@[-1]}" }
bindkey '^I' complete-word
null-line() { echo -E - $'\0' }
compprefuncs=(null-line)
comppostfuncs=(null-line exit)
echo ready
EOF

zmodload zsh/zpty
integer failed=0
# What typing the words and Tab offers, one match to a line.
offered() {
  local line said
  zpty z env -i HOME=$here TERM=xterm ZDOTDIR=$here/dot PATH=$PATH \
    PASSWORDS_SOCKET=$PASSWORDS_SOCKET zsh -d -i
  zpty -r z line '*ready*'
  zpty -w -n z "$1"$'\t'
  while zpty -r z line; do said+=$line; done
  zpty -d z 2>/dev/null
  said=${said//$'\r'/}
  print -r -- "${${said#*$'\0'}%%$'\0'*}" | sed 's/\x1b\[[0-9;?]*[A-Za-z]//g; /^$/d'
}
# Fails unless every one of the matches wanted is among those offered.
expect() {
  local typed=$1 wanted
  local -a got
  shift
  got=("${(@f)$(offered $typed)}")
  for wanted in "$@"; do
    if (( ! ${got[(Ie)$wanted]} )); then
      print -r -- "\"$typed\" didn't offer $wanted; it offered:"
      print -rl -- "  "$^got
      failed=1
    fi
  done
}
# Fails if any of the matches unwanted is among those offered.
refuse() {
  local typed=$1 unwanted
  local -a got
  shift
  got=("${(@f)$(offered $typed)}")
  for unwanted in "$@"; do
    if (( ${got[(Ie)$unwanted]} )); then
      print -r -- "\"$typed\" offered $unwanted, which it shouldn't"
      failed=1
    fi
  done
}

# What the manager has been asked so far.
asked_so_far() { cat $ASKED 2>/dev/null }
# Fails if the manager was asked anything since what it had been asked.
unasked() {
  if [[ "$(asked_so_far)" != "$asked" ]]; then
    print -r -- "A Tab asked the password manager something:"
    diff <(print -r -- "$asked") <(asked_so_far)
    failed=1
  fi
}

asked=$(asked_so_far)
expect 'passwords ' find list details show copy code let-in health audit keys help
expect 'passwords a' audit
expect 'passwords co' copy code
# Nothing's held until something has been read, so no entry is offered.
refuse 'passwords show ' GitHub
unasked
passwords list > /dev/null
asked=$(asked_so_far)
expect 'passwords show ' GitHub 'Bank\ Card' 'Door\ Codes'
expect 'passwords code B' 'Bank\ Card'
expect 'passwords details ' GitHub
# GitHub is held only in summary, so its details aren't known yet.
refuse 'passwords show GitHub ' password
unasked
passwords details GitHub > /dev/null
asked=$(asked_so_far)
expect 'passwords show GitHub ' username password 'one-time\ password'
expect 'passwords copy Git' GitHub
expect 'passwords let-in ' keeping
expect 'passwords list -' --json
expect 'passwords show GitHub -' --json
expect 'passwords audit -' --json
refuse 'passwords let-in keeping -' --json
unasked
exit $failed
