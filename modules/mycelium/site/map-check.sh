# shellcheck shell=bash
# mycelium-map-check: whether the map this machine serves reaches every
# cell of its realm, from the map's own drawing, read in headless Chromium
# (tests/live-reach-test.py). As text it says how many cells the map
# reaches and names each it misses; with --json it prints what the map
# said as one line, {"at","cells","count","failures":[{"cell",
# "orchestration","owner","why"}]}, each why "alone", "unnamed network" or
# "unreachable". It exits 0 when it misses none, 1 when it misses some,
# and 69 when the map never read the node. The browser's own words go to
# standard error. Its help and failures take the shape every command of
# ours does (docs/commands.md), by hand, since a script has no kit.
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
EOF
      exit 0
      ;;
    --json) format=json ;;
    --format=*) format=${1#--format=} ;;
    --format)
      format=${2:-}
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

MAP_CHECK_ROOT=${MAP_CHECK_ROOT:-http://sites.localhost:8484/} MAP_CHECK_SAID=${said} \
  PYTHONPATH=@live_page@ PYTHONDONTWRITEBYTECODE=1 \
  @python@ -m pytest -q -p no:cacheprovider -p live_page --import-mode=importlib \
  --basetemp="${work}/pytest" @test@ >&2 || true

if [[ ! -s ${said} ]]; then
  printf '%s\n' "the map never read the mycelium node" \
    "  the page at ${MAP_CHECK_ROOT:-http://sites.localhost:8484/} drew no reading in time" \
    "  try  mycelium machines, to see whether the node answers" >&2
  exit 69
fi
if [[ ${format} == json ]]; then
  jq -c . "${said}"
else
  jq -r -f /dev/stdin "${said}" << 'EOF'
if .count == 0 then "The map reaches all \(.cells) cells."
else "The map misses \(.count) of \(.cells) cells.",
  (.failures[] | "  \(.cell)  \(.why)\(if .orchestration then "  in \(.orchestration)" else "" end)")
end
EOF
fi
count=$(jq .count "${said}")
[[ ${count} == 0 ]]
