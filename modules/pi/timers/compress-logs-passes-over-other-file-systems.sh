# shellcheck shell=bash
# Runs the nightly log rewrite, the program it's given, over /proc, which
# is never btrfs, in a build's sandbox or out of it, and passes when it
# ends at once with 0, saying there's nothing to compress, rather than
# failing; and when it refuses being asked wrongly, or for a folder that
# isn't there, with the status its help names.
program=$1
said=$("$program" /proc)
status=$?
echo "$said"
if [ "$status" -ne 0 ]; then
  echo "the rewrite ended $status over /proc, not 0" >&2
  exit 1
fi
case $said in
*"isn't on btrfs, so there's nothing to compress") ;;
*)
  echo "the rewrite didn't say it passed over /proc" >&2
  exit 1
  ;;
esac

# Each way of asking, and the status it should end with.
refuses() {
  local want=$1
  shift
  "$program" "$@" > /dev/null 2>&1
  local got=$?
  if [ "$got" -ne "$want" ]; then
    echo "the rewrite asked with [$*] ended $got, not $want" >&2
    exit 1
  fi
}
refuses 64
refuses 64 --bogus
refuses 64 /proc /proc
refuses 66 /no/such/sessions
refuses 0 /proc --help
