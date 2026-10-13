# Completes tuple commands in a real zsh line editor, through zpty, and
# fails on any that doesn't offer what it should. Given the completion and
# the wl-paste it reads the clipboard with; tuple and the clipboard are
# stand-ins printing what Tuple v2026_09_16.1 and a clipboard print, since
# the real tuple asks its daemon. A completion here prints each match, a
# bar and how it's shown, between two NULs, and then the shell exits, so
# it's read to the end.
completion=$1
clipboard=$2
here=$PWD
mkdir -p standins dot
cat > standins/tuple <<'EOF'
#!/bin/sh
case $1 in
  ls) printf '%s\n' \
    '  117559 Aadi Sanghani <aadi@example.com> [offline]' \
    '  9663 Burke Libbey <burke@example.com> [unavailable] (favorite)' \
    '  86826 Henrique Faria <henrique@example.com> [offline]' \
    '  142412 Henrique Squinello <hsq@example.com> [online]' ;;
  settings) printf '%s\n' \
    'overlay | 1 | Show "overlay" annotation window when sharing screen (0|1)' \
    'capture | portal | Screen capture mechanism (auto|x11|portal)' \
    'transcription-model |  | File path to whisper ggml model' ;;
esac
EOF
# The stand-in clipboard is the file, when there is one.
cat > standins/clipboard <<EOF
#!/bin/sh
cat $here/copied 2>/dev/null
EOF
chmod +x standins/*
sed "s|$clipboard|$here/standins/clipboard|" $completion > _tuple
grep -q "$here/standins/clipboard" _tuple || {
  print "the completion doesn't read the clipboard with $clipboard"
  exit 1
}
cat > dot/.zshrc <<EOF
PROMPT=''
# The system's zshenv may set PATH, so the stand-ins go first again.
path=($here/standins \$path)
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
  typeset -a __hits __dscr __tmp
  if (( $@[(I)-d] )); then
    __tmp=${@[$[${@[(i)-d]}+1]]}
    if [[ $__tmp == \(* ]]; then eval "__dscr=$__tmp"; else __dscr=("${(@P)__tmp}"); fi
  fi
  builtin compadd -A __hits -D __dscr "$@"
  [[ -n $__hits ]] || return
  local i
  for i in {1..$#__hits}; do
    print -r -- "$__hits[$i]${__dscr[$i]:+ | $__dscr[$i]}"
  done
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
# Fails unless what's offered for the words is the lines wanted, in order,
# with runs of spaces, which line descriptions up, read as one.
expect() {
  local typed=$1
  local -a got
  shift
  got=("${(@f)$(offered $typed | tr -s ' ')}")
  if [[ ${(j:\n:)got} != ${(j:\n:)@} ]]; then
    print -r -- "\"$typed\" offered:"
    print -rl -- "  "$^got
    print -r -- "  but should have offered:"
    print -rl -- "  "$^@
    failed=1
  fi
}

expect 'tuple sh' 'share | share -- share your screen'
expect 'tuple set ca' 'capture | capture -- Screen capture mechanism, now portal'
expect 'tuple set capture ' auto x11 portal
expect 'tuple set overlay ' 0 1
expect 'tuple favorite henr' \
  '86826 | 86826 Henrique Faria <henrique@example.com>' \
  '142412 | 142412 Henrique Squinello <hsq@example.com>'
expect 'tuple favorite SANG' '117559 | 117559 Aadi Sanghani <aadi@example.com>'
expect 'tuple unfavorite ' '9663 | 9663 Burke Libbey <burke@example.com>'
expect 'tuple join ' '(said) a call link (none is copied)'
print 'Have a look: https://tuple.app/c/Ab3_x-9 when you can' > copied
expect 'tuple join ' 'https://tuple.app/c/Ab3_x-9'
exit $failed
