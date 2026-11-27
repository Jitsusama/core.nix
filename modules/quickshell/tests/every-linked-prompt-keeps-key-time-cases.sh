# shellcheck shell=bash
# The cases of every-linked-prompt-keeps-key-time.py, then the check over the
# shell's own QML, run with bash, a Python, the check and a folder of the
# shell's QML as its arguments (modules/quickshell/checks/command-line.nix).
# So it can't pass by seeing nothing, a linked Prompt given no keyNow and one
# whose keyNow sits only in a child are each shown to fail first, as a folder
# holding no QML is, while a linked Prompt given keyNow and an unlinked one
# given none pass.
set -eu
python=$1 script=$2 shells=$3

check() {
  "$python" "$script" "$@"
}

# What the check must say of a file, and the file's lines.
unkept() {
  told=$1
  shift
  rm -rf quickshell
  mkdir quickshell
  printf '%s\n' "$@" > quickshell/Surface.qml
  if check quickshell > said; then
    echo "The key time check passed on: $*"
    exit 1
  fi
  if ! grep -qF -- "$told" said; then
    echo "Of $* the key time check didn't say: $told"
    cat said
    exit 1
  fi
}

unkept 'Surface.qml:1: Prompt p is linked but given no keyNow' \
  'Prompt {' '    id: p' '}' 'p.linked();'
# The keyNow is a child's, so the prompt itself keeps the wall clock.
unkept 'Surface.qml:2: Prompt p is linked but given no keyNow' \
  'Item {' '    Prompt {' '        id: p' '        Item {' \
  '            keyNow: () => KeyTimes.now()' '        }' '    }' \
  '    function link() { p.linked(); }' '}'
rm -rf quickshell
mkdir quickshell
if check quickshell > said; then
  echo "The key time check passed on a folder holding no QML"
  exit 1
fi
# A linked Prompt given keyNow passes, as one no link opens given none does.
printf '%s\n' 'Item {' '    Prompt {' '        id: opened' \
  '        keyNow: () => KeyTimes.now()' '    }' '    Prompt { id: asked }' \
  '    function link() { opened.linked(); }' '}' > quickshell/Surface.qml
check quickshell
check "$shells"
