# shellcheck shell=bash
# Checks every platform profile Dell's driver offers is one the SoC slider
# takes too, since the legacy profile file, the one power-profiles-daemon
# writes, offers only what every profile driver takes.
# usage: platform-profiles-reach-the-legacy-file.sh <Dell's driver> <the SoC slider>
set -euo pipefail

# The profiles a driver says it offers, one a line.
profiles() {
  { grep -o 'set_bit(PLATFORM_PROFILE_[A-Z_]*' "$1" || true; } | cut -d '(' -f 2 | sort -u
}

offered=$(profiles "$1")
if [ -z "$offered" ]; then
  echo "Dell's profile driver offers no profile, so there's nothing to hold."
  exit 1
fi
taken=$(profiles "$2")
missing=
for profile in $offered; do
  grep -qx "$profile" <<< "$taken" || missing="$missing $profile"
done
if [ -n "$missing" ]; then
  echo "The SoC slider doesn't take profiles Dell's driver offers:$missing"
  exit 1
fi
