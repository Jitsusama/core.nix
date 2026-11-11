# shellcheck shell=bash
# Checks every setting asked of a kernel is in its finished configuration,
# since Kconfig quietly drops a setting whose dependencies aren't met. An
# optional setting also holds when the kernel doesn't offer its option at
# all. Each setting is a line of the settings file: the line it leaves in
# the configuration, its option, and "optional" or "required", by tabs.
# usage: kernel-settings-hold.sh <configuration> <settings>
set -euo pipefail

config=$1
failed=
while IFS=$'\t' read -r line option kind; do
  grep -qxF "$line" "$config" && continue
  [ "$kind" = optional ] && ! grep -qE "^(# )?CONFIG_${option}[= ]" "$config" && continue
  echo "Kernel setting didn't hold: $line"
  failed=1
done < "$2"
[ -z "$failed" ]
