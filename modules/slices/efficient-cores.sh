# shellcheck shell=bash
# Holds the efficient cores' clock ceiling at a given speed, or frees each
# to its highest clock. The processor shares one power budget among its
# cores, so with every core busy each runs near the same clock; held
# lower, the efficient ones leave the performance cores the budget
# (nixos.nix says why). Writing a ceiling needs root, so
# efficient-cores-held@.service runs it.
#
# One hold stands at a time: it writes down its speed before it holds,
# under a lock, so a second hold at another speed is refused even when
# both start together, and a hold that failed partway is freed in full.
# The note goes only once every core is freed, so a free the kernel
# refused partway can be asked again.
#
# Freeing writes each core's highest clock rather than the ceiling read
# before the hold, because the kernel reads back the lowest of every
# limit on a core, a thermal one among them, and writing that back would
# keep it as ours once it lifted. The kernel keeps each other limit on
# its own, so freeing ours leaves theirs standing.
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand. EFFICIENT_CORES_SYSFS puts sysfs under
# another root and EFFICIENT_CORES_RUN its notes elsewhere, for the test
# alone.
set -euo pipefail
sys=${EFFICIENT_CORES_SYSFS:-}/sys
run=${EFFICIENT_CORES_RUN:-/run/efficient-cores}
atom=$sys/devices/cpu_atom/cpus

refuse() {
  printf '%s\n' "efficient-cores $1" "  try  efficient-cores --help" >&2
  exit 64
}

# A speed from 400 to 5000 MHz, which spans every core's clock today and
# is the range efficient-cores.rules grants, written without a leading
# zero, which bash would read as octal.
speed() {
  if [[ ! "$1" =~ ^[1-9][0-9]{2,3}$ ]] || ((10#$1 < 400 || 10#$1 > 5000)); then
    refuse "holds a speed from 400 to 5000 MHz, not $1"
  fi
}

# The efficient cores, one number a line, from the kernel's list, such as
# "4-15" or "2-3,5".
efficient() {
  local part
  local -a parts
  IFS=, read -r -a parts <"${atom}"
  for part in "${parts[@]}"; do
    if [[ "${part}" == *-* ]]; then
      seq "${part%-*}" "${part#*-}"
    else
      echo "${part}"
    fi
  done
}

cpu=$sys/devices/system/cpu

# Holds and frees run one at a time, since systemd starts and stops each
# speed's unit on its own.
lock() {
  mkdir -p "${run}"
  exec 9>"${run}/lock"
  flock 9
}

hold() {
  local mhz=$1 cores n low high kHz said=""
  if [[ ! -r "${atom}" ]]; then
    echo "efficient-cores: the kernel names no efficient cores here, so nothing is held" >&2
    exit 69
  fi
  cores=$(efficient)
  if [[ -e "${run}/held" ]]; then
    local other
    other=$(<"${run}/held")
    if [[ "${other}" != "${mhz}" ]]; then
      printf '%s\n' "efficient-cores: already held at ${other} MHz" \
        "  stop efficient-cores-held@${other} first, or start and stop it if it's stopped" >&2
      exit 75
    fi
  else
    echo "${mhz}" >"${run}/held"
  fi
  while read -r n; do
    low=$(<"${cpu}/cpu${n}/cpufreq/cpuinfo_min_freq")
    high=$(<"${cpu}/cpu${n}/cpufreq/cpuinfo_max_freq")
    kHz=$((mhz * 1000))
    if ((kHz < low)); then
      kHz=${low}
    elif ((kHz > high)); then
      kHz=${high}
    fi
    echo "${kHz}" >"${cpu}/cpu${n}/cpufreq/scaling_max_freq"
    said+=" cpu${n} $((kHz / 1000))"
  done <<<"${cores}"
  echo "efficient-cores: held at most, in MHz:${said}"
}

# free [MHZ]: free each efficient core to its highest clock. Given a
# speed, as a unit's stop gives its own, it leaves a hold at another speed
# alone.
free() {
  local mhz=${1:-} cores n
  if [[ ! -e "${run}/held" ]]; then
    echo "efficient-cores: nothing is held"
    return
  fi
  if [[ -n "${mhz}" && "$(<"${run}/held")" != "${mhz}" ]]; then
    echo "efficient-cores: the hold is at $(<"${run}/held") MHz, not ${mhz}, so it stays"
    return
  fi
  if [[ -r "${atom}" ]]; then
    cores=$(efficient)
    while read -r n; do
      cp "${cpu}/cpu${n}/cpufreq/cpuinfo_max_freq" "${cpu}/cpu${n}/cpufreq/scaling_max_freq"
    done <<<"${cores}"
  fi
  rm "${run}/held"
  echo "efficient-cores: each efficient core is free to its highest clock"
}

case "${1-}" in
-h | --help)
  cat <<'EOF'
efficient-cores  holds or frees the efficient cores' clock

EXAMPLES
  efficient-cores hold MHZ  hold each at most MHZ, from 400
                            to 5000, within its own range
  efficient-cores free      free each to its highest clock
  efficient-cores free MHZ  the same, if the hold is MHZ's

EXIT
  0   done
  1   the kernel refused a setting
  64  asked wrongly
  69  the kernel names no efficient cores
  75  another hold stands
EOF
  ;;
hold)
  [[ $# -eq 2 ]] || refuse "hold takes one speed, in MHz"
  speed "$2"
  lock
  hold "$2"
  ;;
free)
  [[ $# -le 2 ]] || refuse "free takes at most a speed"
  [[ $# -eq 1 ]] || speed "$2"
  lock
  free "${2-}"
  ;;
*)
  refuse "holds or frees the clock, not ${1:-nothing}"
  ;;
esac
