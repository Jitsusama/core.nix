# shellcheck shell=bash # The check runs it with bash.
# Runs swap-file-grows.sh, the script named first, on a swap file in a
# scratch folder, with stand-ins for btrfs, which makes the file at the
# size it's told, and for df, which says the room it's given. It grows a
# smaller file and leaves every other alone.
set -euo pipefail

grows=$1
work=$(mktemp -d)
mkdir "$work/bin"

# Makes the file sparse, so a size in gibibytes costs nothing. Told to
# fail, it fails for a size given with a unit, not for one in bytes.
cat >"$work/bin/btrfs" <<EOF
#!$BASH
if [ -e "$work/fails" ] && [[ \$4 == *[A-Za-z] ]]; then
  exit 1
fi
truncate -s "\$(numfmt --from=iec "\$4")" "\$5"
EOF
cat >"$work/bin/df" <<EOF
#!$BASH
printf 'Avail\n%s\n' "\$FREE"
EOF
chmod +x "$work/bin/btrfs" "$work/bin/df"

# A swap file of 1 GiB, not in use, with nothing hibernated to it.
fresh() {
  rm -rf "$work/fails" "$work/efivars"
  mkdir "$work/efivars"
  : >"$work/swaps"
  rm -f "$work/swapfile"
  truncate -s 1G "$work/swapfile"
}

grow() {
  SWAPS=$work/swaps EFIVARS=$work/efivars PATH="$work/bin:$PATH" \
    FREE=${FREE:-$((100 * 1024 * 1024 * 1024))} bash "$grows" "$work/swapfile" "$1"
}

expect() {
  local size
  size=$(stat -c %s "$work/swapfile")
  if [ "$size" != "$(numfmt --from=iec "$1")" ]; then
    echo "$2, but the swap file is $size bytes, not $1"
    exit 1
  fi
}

fresh
grow 2G
expect 2G "a smaller swap file grows"

fresh
grow 1G
expect 1G "one already the size it's asked for stays"

fresh
grow 512M
expect 1G "a bigger one isn't shrunk"

fresh
echo "$work/swapfile file 1048572 0 -2" >"$work/swaps"
grow 2G
expect 1G "one in use stays as it is"

fresh
touch "$work/efivars/HibernateLocation-8cf3337e-9f1a-4a1b-8c3a-2f9a5d1e8c11"
grow 2G
expect 1G "one that may hold a hibernation stays as it is"

fresh
FREE=$((10 * 1024 * 1024 * 1024)) grow 2G
expect 1G "one the disk hasn't room to grow stays as it is"

fresh
touch "$work/fails"
grow 2G
expect 1G "one that couldn't grow is made again at the size it was"
