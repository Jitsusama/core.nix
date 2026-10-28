# shellcheck shell=bash
# Checks lock-for-sleep, given stand-ins for lock-screen and niri that write
# what they're asked: the lock is drawn before the screens go off, and they
# go off even when the lock wasn't drawn in time, which still fails.
# usage: lock-for-sleep-test.sh <lock-for-sleep.sh> <a folder for the stand-ins>
set -euo pipefail

script=$1
stand_ins=$2
said=$stand_ins/said
mkdir -p "$stand_ins/bin"
export STAND_IN_SAID=$said

fails() {
  echo "lock-for-sleep $1" >&2
  exit 1
}

cat > "$stand_ins/bin/lock-screen" << 'EOF'
echo lock >> "$STAND_IN_SAID"
exit "${LOCK_STATUS:-0}"
EOF
cat > "$stand_ins/bin/niri" << 'EOF'
echo "niri $*" >> "$STAND_IN_SAID"
EOF
for stand_in in lock-screen niri; do
  sed -i "1i #!$BASH" "$stand_ins/bin/$stand_in"
  chmod +x "$stand_ins/bin/$stand_in"
done
export PATH=$stand_ins/bin:$PATH

order="lock
niri msg action power-off-monitors"

LOCK_STATUS=0 "$BASH" "$script" || fails "failed though the lock was drawn"
[[ $(cat "$said") == "$order" ]] || fails "didn't lock and then turn the screens off: $(cat "$said")"

rm "$said"
status=0
LOCK_STATUS=1 "$BASH" "$script" || status=$?
[[ $status == 1 ]] || fails "didn't fail when the lock wasn't drawn in time"
[[ $(cat "$said") == "$order" ]] || fails "didn't turn the screens off when the lock was late: $(cat "$said")"
