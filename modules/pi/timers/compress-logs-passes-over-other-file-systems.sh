# shellcheck shell=bash
# Runs the nightly log rewrite, the program it's given, over /proc, which
# is never btrfs, in a build's sandbox or out of it, and passes when it
# ends at once saying there's nothing to compress rather than failing.
program=$1
said=$("$program" /proc)
echo "$said"
case $said in
*"isn't on btrfs, so there's nothing to compress") ;;
*)
  echo "the rewrite didn't say it passed over /proc" >&2
  exit 1
  ;;
esac
