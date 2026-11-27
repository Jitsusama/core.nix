# Completes tuple commands in a real zsh line editor, through zpty, with
# the completion Tuple's own command writes and the real tuple, which zsh
# asks what each word can be, and fails unless each start offers the
# commands it should. A completion here prints each match, a bar and how
# it's shown, between two NULs, and then the shell exits, so it's read to
# the end. Only the matches are held to, since how Tuple describes each is
# its own to change.
completion=$1
here=$PWD
mkdir -p dot
cp $completion _tuple
cat > dot/.zshrc <<EOF
PROMPT=''
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
# What typing the words and Tab offers, one match to a line. Each editor
# gets the build's runtime and data folders, so a daemon its tuple starts
# keeps everything where tuple-completes-in-zsh.sh's stop finds it.
offered() {
  local line said
  zpty z env -i HOME=$here TERM=xterm ZDOTDIR=$here/dot PATH=$PATH \
    XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR XDG_DATA_HOME=$XDG_DATA_HOME zsh -d -i
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
  # Named first, so a run cut off for time says which editor hung
  # (tuple-completes-in-zsh.sh).
  print -ru2 -- "typing \"$typed\""
  got=("${(@f)$(offered $typed)}")
  for wanted in "$@"; do
    if (( ! ${got[(Ie)$wanted]} )); then
      print -r -- "\"$typed\" didn't offer $wanted; it offered:"
      print -rl -- "  "$^got
      failed=1
    fi
  done
}

expect 'tuple ca' call capture
# Among these, every command core runs against it, and the flags
# tuple-call and tuple-people pass as they follow it: --format before the
# command, as they pass it, and --raw after state follow.
expect 'tuple --' --format
expect 'tuple call ' accept join leave mute unmute start
expect 'tuple call l' leave list
expect 'tuple cont' contacts
expect 'tuple contacts ' favorite follow unfavorite
# Tuple calls these two deprecated, so one failing may mean a build only
# hides it from Tab, or that it's gone. Never try one by hand to tell:
# new opens a call in Joel's room and share shares his screen.
expect 'tuple ne' new
expect 'tuple sh' share
expect 'tuple st' start state stop
expect 'tuple state ' follow
expect 'tuple state follow --' --raw
# Only v2026_11_19.0 on has webcam, so an older build installed fails here.
expect 'tuple w' webcam whoami
expect 'tuple webcam ' list start stop
exit $failed
