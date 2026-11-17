# shellcheck shell=bash
# mycelium-map-check: whether the map this machine serves reaches every
# cell of its realm, from the map's own drawing, read in headless Chromium
# (tests/live-reach-test.py). As text it says how many cells the map
# reaches and names each it misses; with --json it prints what the map
# said as one line, {"at","cells","count","failures":[{"cell",
# "orchestration","owner","why"}]}, each why "alone", "unnamed network" or
# "unreachable". It exits 0 when it misses none, 1 when it misses some,
# 69 when the map never read the node and 70 when the check itself broke.
# The browser's own words go to standard error only when the check broke,
# and a name a peer gave is printed as text with its control characters
# made spaces. Its help and failures take the shape every command of ours
# does (docs/commands.md), by hand, since a script has no kit.
#
# A reading takes ten seconds or more on a loaded machine, so as text it
# first says on standard error what the last one found and when, then
# that it's looking again; every answer is kept for the next run, in the
# state folder's mycelium/map-check.json. As JSON it says nothing on the
# way, for the readers that count with it.
#
# The test runs from a copy in the run's own folder, since pytest roots
# itself at the folder of the file it's given and lists all of it first:
# in the Nix store, a minute's work under load for a ten-second check.
#
# MAP_CHECK_ROOT, the sites' root, defaults to this machine's.
# Given by Nix: @python@, @test@ and @live_page@.

format=text
while [[ ${#} -gt 0 ]]; do
  case ${1} in
    -h | --help)
      cat << 'EOF'
mycelium-map-check  whether the map reaches every cell

  Opens the mycelium map this machine serves in a headless
  browser and asks its drawing which cells no step in, fold
  or find reaches, which cells of a network it draws alone,
  and which networks it names by address alone.

EXAMPLES
  mycelium-map-check          how many cells it reaches
  mycelium-map-check --json   what it misses, as JSON

OPTIONS
  --format text|json   the answer's form; text by default
  --json               the same as --format json

EXIT
  0   the map reaches every cell
  1   the map misses some
  64  asked wrongly
  69  the map never read the mycelium node
  70  the check itself broke
EOF
      exit 0
      ;;
    --json) format=json ;;
    --format=*) format=${1#--format=} ;;
    --format)
      if [[ ${#} -lt 2 ]]; then
        printf '%s\n' "--format needs a value" "  the formats are text and json" \
          "  try  mycelium-map-check --help" >&2
        exit 64
      fi
      format=${2}
      shift
      ;;
    *)
      printf '%s\n' "no option called ${1}" "  it takes --format and --json" \
        "  try  mycelium-map-check --help" >&2
      exit 64
      ;;
  esac
  shift
done
case ${format} in
  text | json) ;;
  *)
    printf '%s\n' "no format called ${format}" "  the formats are text and json" \
      "  try  mycelium-map-check --help" >&2
    exit 64
    ;;
esac

work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
said=${work}/said.json
root=${MAP_CHECK_ROOT:-http://sites.localhost:8484/}
kept=${XDG_STATE_HOME:-${HOME}/.local/state}/mycelium/map-check.json

# What the last reading found, said while this one is made.
if [[ ${format} == text ]]; then
  if last=$(jq -r -f /dev/stdin "${kept}" 2> /dev/null << 'EOF'
(.at | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601 | localtime | strftime("%H:%M")) as $at
| if .count == 0 then "At \($at) the map reached all \(.cells) cells."
  else "At \($at) the map missed \(.count) of \(.cells) cells." end
EOF
  ); then
    printf '%s\n' "${last}" >&2
  fi
  printf '%s\n' "Opening the map in a headless browser to look again." >&2
fi

# From its own folder, so no Python where it was asked from runs, and on a
# copy of the test alone in a folder of its own, so pytest lists only that.
cd "${work}" || exit 70
mkdir "${work}/test" && cp @test@ "${work}/test/live-reach-test.py" || exit 70
MAP_CHECK_ROOT=${root} MAP_CHECK_SAID=${said} \
  PYTHONPATH=@live_page@ PYTHONDONTWRITEBYTECODE=1 \
  @python@ -m pytest -q -p no:cacheprovider -p live_page --import-mode=importlib \
  --rootdir="${work}/test" --basetemp="${work}/pytest" "${work}/test/live-reach-test.py" \
  > "${work}/pytest.log" 2>&1 || true

if [[ ! -s ${said} ]]; then
  if grep -q "the map never read the node" "${work}/pytest.log"; then
    printf '%s\n' "the map never read the mycelium node" \
      "  the page at ${root} drew no live reading in time" \
      "  try  systemctl --user status local-sites, then mycelium machines" >&2
    exit 69
  fi
  # The browser's words are said only when the check broke, to say where.
  cat "${work}/pytest.log" >&2
  printf '%s\n' "the check broke before the map said its reach" \
    "  the browser's words above say where" \
    "  try  mycelium-map-check again, and tell the map's owner if it repeats" >&2
  exit 70
fi
# Kept for the next run to open with; a run that can't keep it still answers.
mkdir -p "${kept%/*}" && cp "${said}" "${kept}" 2> /dev/null || true
if [[ ${format} == json ]]; then
  jq -c . "${said}"
else
  jq -r -f /dev/stdin "${said}" << 'EOF'
def plain: tostring | gsub("[\u0000-\u001f\u007f-\u009f]"; " ");
if .count == 0 then "The map reaches all \(.cells) cells."
else "The map misses \(.count) of \(.cells) cells.",
  (.failures[] | "  \(.cell | plain)  \(.why)"
    + (if .orchestration then "  in \(.orchestration | plain)" else "" end))
end
EOF
fi
count=$(jq .count "${said}")
[[ ${count} == 0 ]]
