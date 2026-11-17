# shellcheck shell=bash
# Checks mycelium-map-check, given as $1, built on a stand-in for its browser
# test that records what was beside the test and what had been said on the
# way when it was opened. Before the browser opens, it says what the kept
# reading found and when, here rather than in UTC, with the day when it
# wasn't today, then that it's looking again; it keeps each answer whole; it
# hands pytest the test alone in a folder of its own; as JSON it says nothing
# on the way; a kept file no reading writes, or none, says only that it's
# looking; and with no home at all it still answers.
set -o errexit -o nounset -o pipefail

check=$1
# Five hours behind, so a time said in UTC rather than here is caught.
export TZ=EST5 XDG_STATE_HOME=${TMPDIR}/state MAP_CHECK_BESIDE=${TMPDIR}/beside
export MAP_CHECK_ERR=${PWD}/err MAP_CHECK_BEFORE=${TMPDIR}/before
kept=${XDG_STATE_HOME}/mycelium/map-check.json
looking="Opening the map in a headless browser to look again."
mkdir -p "${kept%/*}"

# Runs the check and fails, saying $2, unless before the browser opened it
# had said $1.
opens() {
  "${check}" > out 2> err
  if [[ $(< "${MAP_CHECK_BEFORE}") != "${1}" ]]; then
    echo "${2}; before the browser opened it said:" >&2
    cat "${MAP_CHECK_BEFORE}" >&2
    exit 1
  fi
}

printf '%s' '{"at":"2026-01-02T13:05:22.164Z","cells":5,"count":2,"failures":[]}' > "${kept}"
opens "At 08:05 on 2 January the map missed 2 of 5 cells."$'\n'"${looking}" \
  "it didn't say the last answer, here and with its day, then that it's looking"
[[ $(< out) == "The map reaches all 3 cells." ]] || {
  echo "its answer was: $(< out)" >&2
  exit 1
}
cells=$(jq .cells "${kept}")
[[ ${cells} == 3 ]] || {
  echo "it didn't keep its answer" >&2
  exit 1
}
half=$(find "${kept%/*}" -name '*.json.*')
[[ -z ${half} ]] || {
  echo "it left a half-kept file" >&2
  exit 1
}
if [[ $(< "${MAP_CHECK_BESIDE}") != live-reach-test.py ]]; then
  beside=$(wc -l < "${MAP_CHECK_BESIDE}")
  echo "pytest was handed a test beside ${beside} entries, not alone" >&2
  exit 1
fi
opens "At 09:09 on 2 January the map reached all 3 cells."$'\n'"${looking}" \
  "it didn't say the answer it kept"

now=$(date +%s)
at=$(date -u -d "@${now}" +%FT%T.5Z)
here=$(date -d "@${now}" +%H:%M)
jq -n --arg at "${at}" '{at:$at,cells:4,count:1,failures:[]}' > "${kept}"
opens "At ${here} the map missed 1 of 4 cells."$'\n'"${looking}" \
  "a reading from today didn't give just its time"

"${check}" --json > out 2> err
[[ ! -s err ]] || {
  echo "as JSON it said on the way: $(< err)" >&2
  exit 1
}
jq -e '.cells == 3' out > /dev/null

for unread in 'not a reading' '{"at":"2026-01-02T13:05:22Z","cells":"\u001b[2J","count":0}'; do
  printf '%s' "${unread}" > "${kept}"
  opens "${looking}" "a kept file no reading writes wasn't left unsaid"
done
rm "${kept}"
opens "${looking}" "with nothing kept, it said more than that it's looking"

env -u XDG_STATE_HOME -u HOME "${check}" > out 2> err || {
  echo "with no home, it broke: $(< err)" >&2
  exit 1
}
[[ $(< out) == "The map reaches all 3 cells." ]] || {
  echo "with no home it said: $(< out)" >&2
  exit 1
}
