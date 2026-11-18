# shellcheck shell=bash
# What open-now says two Neovims in one kitty hold, for
# the-launcher-acts-only-on-what-it-can, as open-with's neovim-held.lua has
# them say it: the first with two tabs, a file modified and one hidden, the
# second with one tab and a file the first holds too, modified there.
set -euo pipefail

[[ $1 == neovim ]] || exit 1
file() { printf '{"buffer":%s,"path":"%s","modified":%s,"tabs":[%s],"at":%s}' "$@"; }
main=$(file 1 "$HOME/src/alpha/main.rs" false 1 100)
notes=$(file 2 "$HOME/src/alpha/docs/notes.md" true 2 200)
env=$(file 3 "$HOME/src/alpha/.env" false "" 50)
readme=$(file 4 "$HOME/src/beta/README.md" false 1 300)
changed=$(file 1 "$HOME/src/alpha/main.rs" true 1 10)
printf '@5\t41:7\t/run/n1\t{"pid":101,"folder":"%s","tab":2,"tabs":2,"current":2,' \
  "$HOME/src/alpha"
printf '"files":[%s,%s,%s]}\n' "$main" "$notes" "$env"
printf '@6\t41:8\t/run/n2\t{"pid":102,"folder":"%s","tab":1,"tabs":1,"current":4,' \
  "$HOME/src/beta"
printf '"files":[%s,%s]}\n' "$readme" "$changed"
