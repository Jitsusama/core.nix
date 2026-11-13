# Completes mycelium's swarm in a real zsh line editor, through zpty, with
# the completion beside the module and a stand-in mycelium on the PATH
# that names the swarm's teams as the node would, and fails unless each
# start offers what it should. A completion here prints each match, and
# then the shell exits, so it's read to the end, as eero's test reads it.
completion=$1
here=$PWD
mkdir -p dot bin
cp $completion _mycelium
# The stand-in answers only `swarm --names`, with a team of the realm the
# line names, so a completion asking the wrong realm offers the wrong one.
cat > bin/mycelium <<'EOF'
#!/bin/sh
case " $* " in
  *" --realm work swarm --names "*) printf 'kelp\n' ;;
  *" swarm --names "*) printf 'part-deux\ndelta\n' ;;
esac
EOF
chmod +x bin/mycelium
PATH=$here/bin:$PATH
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
  zpty z env -i HOME=$here TERM=xterm ZDOTDIR=$here/dot PATH=$PATH zsh -d -i
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

expect 'mycelium swarm ' say pause resume part-deux delta
expect 'mycelium swarm pa' pause part-deux
refuse 'mycelium swarm pa' delta say
# After an act comes its team; after a team, what to do to it.
expect 'mycelium swarm say ' part-deux delta
refuse 'mycelium swarm say ' say pause resume
expect 'mycelium swarm delta ' say pause resume
refuse 'mycelium swarm delta ' part-deux
# What's said is the person's own words, so nothing is offered for them.
refuse 'mycelium swarm say delta ' part-deux delta say
# The teams are the realm's the line names.
expect 'mycelium --realm work swarm ' kelp
refuse 'mycelium --realm work swarm ' part-deux
exit $failed
