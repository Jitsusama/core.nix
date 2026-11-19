# shellcheck shell=bash
# Holds the efficient cores' clock ceiling at a given speed, or frees it to
# each core's full clock. The processor shares one power budget among its
# cores, so with every core busy each runs near the same clock; held lower,
# the efficient ones leave the performance cores the budget (nixos.nix says
# why). Writing a ceiling needs root, so efficient-cores-held@.service runs
# it. EFFICIENT_CORES_SYSFS puts sysfs elsewhere, for the test alone.
set -euo pipefail
sys=${EFFICIENT_CORES_SYSFS:-}/sys

usage() {
  cat <<'EOF'
efficient-cores hold MHZ   hold each efficient core's ceiling at MHZ, within its own range
efficient-cores free       free each efficient core's ceiling to its full clock
EOF
}

refuse() {
  echo "efficient-cores: $1" >&2
  usage >&2
  exit 64
}

# The efficient cores, one number a line, from the kernel's list, such as
# "4-15" or "2-3,5".
efficient() {
  local part
  local -a parts
  IFS=, read -r -a parts <"$sys/devices/cpu_atom/cpus"
  for part in "${parts[@]}"; do
    if [[ "${part}" == *-* ]]; then
      seq "${part%-*}" "${part#*-}"
    else
      echo "${part}"
    fi
  done
}

# ceiling <kHz, or empty for each core's full clock>
ceiling() {
  local want=$1 n freq low high kHz held=0
  if [[ ! -r "$sys/devices/cpu_atom/cpus" ]]; then
    echo "efficient-cores: this processor has no efficient cores, so it holds nothing"
    return
  fi
  while read -r n; do
    freq=$sys/devices/system/cpu/cpu$n/cpufreq
    low=$(<"${freq}/cpuinfo_min_freq")
    high=$(<"${freq}/cpuinfo_max_freq")
    kHz=${want:-${high}}
    if ((kHz < low)); then
      kHz=${low}
    elif ((kHz > high)); then
      kHz=${high}
    fi
    echo "${kHz}" >"${freq}/scaling_max_freq"
    held=$((held + 1))
  done < <(efficient)
  if [[ -n "${want}" ]]; then
    echo "efficient-cores: ${held} efficient cores held at most ${want} kHz"
  else
    echo "efficient-cores: ${held} efficient cores free to their full clock"
  fi
}

case "${1-}" in
--help | -h)
  usage
  ;;
hold)
  [[ $# -eq 2 ]] || refuse "hold takes one speed, in MHz"
  [[ "$2" =~ ^[0-9]+$ ]] || refuse "'$2' isn't a speed in MHz"
  ceiling "$(($2 * 1000))"
  ;;
free)
  [[ $# -eq 1 ]] || refuse "free takes nothing more"
  ceiling ""
  ;;
*)
  refuse "say hold or free"
  ;;
esac
