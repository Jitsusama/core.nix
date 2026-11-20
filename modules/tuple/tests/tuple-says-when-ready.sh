# shellcheck shell=bash
# Runs tuple.service's own ExecStartPost with systemctl and systemd-cat
# swapped for stand-ins: it must return while the unit is still starting,
# then say the time once the unit is active, in the words modules/health
# reads, timed from a fresh start or from the crash before a restart; and
# say nothing when systemd gives it no start to time from.
#
# Given the ExecStartPost's program, and the systemctl and systemd-cat it
# names.
set -euo pipefail

post=$1
systemctl=$2
journal=$3
here=$PWD

mkdir -p bin props
# Answers each property from props/, and ActiveState as active only once
# the case has said the unit is up.
cat >bin/systemctl <<EOF
#!${BASH}
for arg; do
  case \$arg in --property=*) property=\${arg#--property=} ;; esac
done
if [[ \$property == ActiveState ]]; then
  if [[ -e ${here}/up ]]; then echo active; else echo activating; fi
else
  cat ${here}/props/\$property
fi
EOF
# Keeps what it was asked to write, whole, once it has it all.
cat >bin/systemd-cat <<EOF
#!${BASH}
{ printf '%s\n' "\$*"; cat; } >${here}/said.part
mv ${here}/said.part ${here}/said
EOF
chmod +x bin/systemctl bin/systemd-cat

sed "s|${systemctl}|${here}/bin/systemctl|; s|${journal}|${here}/bin/systemd-cat|" \
  "${post}" >started
chmod +x started
grep -q "${here}/bin/systemctl" started
grep -q "${here}/bin/systemd-cat" started

failed=0

# Sets the unit's stamps, starts the post, and reads what it said once up.
says() {
  local what=$1 wanted=$4
  rm -f said up
  echo "$2" >props/InactiveExitTimestampMonotonic
  echo "$3" >props/ActiveExitTimestampMonotonic
  echo 1343000 >props/ActiveEnterTimestampMonotonic
  ./started tuple.service
  if [[ -e said ]]; then
    echo "${what}: said it before the unit was active"
    failed=1
  fi
  touch up
  for _ in {1..100}; do
    [[ -e said ]] && break
    sleep 0.05
  done
  local got
  got=$(cat said 2>/dev/null || true)
  if [[ ${got} != "${wanted}" ]]; then
    echo "${what}: wanted"
    printf '  %s\n' "${wanted}"
    echo "but said"
    printf '  %s\n' "${got}"
    failed=1
  fi
}

says "a start" 1000000 0 $'--identifier=tuple\ntuple: ready in 343 ms'
says "a restart after a crash" 1000000 1093000 $'--identifier=tuple\ntuple: ready in 250 ms'

rm -f said
echo 0 >props/InactiveExitTimestampMonotonic
echo 0 >props/ActiveExitTimestampMonotonic
./started tuple.service
sleep 0.5
if [[ -e said ]]; then
  echo "with no start to time from, it said: $(cat said)"
  failed=1
fi

exit "${failed}"
