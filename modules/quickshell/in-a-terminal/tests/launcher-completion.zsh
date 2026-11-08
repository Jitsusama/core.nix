# Completes launcher commands in a real zsh line editor, through zpty, with
# the completion `launcher completion zsh` writes and the real launcher,
# which asks the stand-in qs ($LAUNCHER_QS) what each place holds, and
# fails unless each start offers what it should. A completion here prints
# each match between two NULs, and then the shell exits, so it's read to
# the end. Only the matches are held to, since how each is described is
# the shell's to say.
completion=$1
here=$PWD
mkdir -p dot
cp $completion _launcher
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
    LAUNCHER_QS=$LAUNCHER_QS zsh -d -i
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

expect 'launcher ' show do
expect 'launcher show ' now machine sessions-to-reopen
expect 'launcher show m' machine
refuse 'launcher show m' now
expect 'launcher show machine ' disk power
# The flags are taken anywhere, so a step after one still walks.
expect 'launcher show --json machine ' disk power
expect 'launcher show machine -' --drawers --json
# A step that names nothing offers nothing, and says nothing over the line.
refuse 'launcher show zebra ' disk now
# do walks the same rows, and after its -- what's typed is an act's words.
expect 'launcher do machine ' disk power
expect 'launcher do machine -' --yes
refuse 'launcher do machine -- ' disk power
exit $failed
