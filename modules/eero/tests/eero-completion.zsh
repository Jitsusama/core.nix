# Completes eero commands in a real zsh line editor, through zpty, with the
# completion `eero completion zsh` writes and the real eero, which names
# the nodes, devices and profiles of the record in $XDG_RUNTIME_DIR, and
# fails unless each start offers what it should. A completion here prints
# each match, a bar and how it's shown, between two NULs, and then the
# shell exits, so it's read to the end. Only the matches are held to, since
# how each is described is eero's to change.
completion=$1
here=$PWD
mkdir -p dot
cp $completion _eero
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
    XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR zsh -d -i
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

# Completion follows the tree as help lists it: each collection once, and
# no old spelling.
expect 'eero ' status nodes mesh devices profiles internet speeds firmware radios \
  traffic events sign-in
refuse 'eero ' node device profile radio watch speedtest sqm
expect 'eero de' devices
refuse 'eero de' device
# The word after a collection is a name.
expect 'eero nodes ' Hall Study Den
expect 'eero nodes S' Study
# A name is offered as it's typed, its spaces escaped.
expect 'eero devices ' 'tablet\ 1' 02:00:00:00:00:03 02:00:00:00:00:05
# Two devices share a name, so it's offered only as each one's address.
refuse 'eero devices ' 'phone\ 1'
# Nor is a name one device has and another's hostname shares.
expect 'eero devices ' 02:00:00:00:00:0d
refuse 'eero devices ' 'thermostat\ 1'
# A device not on now is offered too, since its page says where it last was,
# and by its address where one on now shares its name.
expect 'eero devices ' 'phone\ 2' 02:00:00:00:00:19
expect 'eero profiles ' 'profile\ 1' 'profile\ 2' 'profile\ 3'
# After a name, what it holds.
expect 'eero nodes Study ' devices radios
expect 'eero profiles profile\ 1 ' devices
expect 'eero internet ' sqm
expect 'eero internet sqm ' on off
expect 'eero speeds ' run
# The old spellings still complete as they did.
expect 'eero node ' Hall Study Den
expect 'eero device ' 'tablet\ 1'
expect 'eero watch ' status devices nodes
expect 'eero watch node ' Hall Study Den
expect 'eero sqm ' on off
expect 'eero devices -' --json --watch
expect 'eero nodes Hall -' --json
exit $failed
