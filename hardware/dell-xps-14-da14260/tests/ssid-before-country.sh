# shellcheck shell=bash
# Checks that cfg80211 records the name of the network a card joined before
# it looks for the access point's country, since a join whose access point
# names none ends there, and networkd matches a network's file on that name.
# usage: ssid-before-country.sh <net/wireless/sme.c>
set -euo pipefail

result=$(awk '/^void __cfg80211_connect_result\(/,/^}$/' "$1")
if [ -z "$result" ]; then
  echo "cfg80211 has no __cfg80211_connect_result to hold."
  exit 1
fi
named=$(grep -n -m1 'wdev->u\.client\.ssid_len = ssid_len;' <<<"$result" | cut -d: -f1 || true)
country=$(grep -n -m1 'WLAN_EID_COUNTRY' <<<"$result" | cut -d: -f1 || true)
if [ -z "$country" ]; then
  echo "cfg80211 no longer looks for the access point's country where this check reads."
  exit 1
fi
if [ -z "$named" ]; then
  echo "cfg80211 no longer records the network's name where this check reads."
  exit 1
fi
if [ "$named" -gt "$country" ]; then
  echo "A network whose access point names no country is joined without its name," \
    "so networkd can't match its file."
  exit 1
fi
