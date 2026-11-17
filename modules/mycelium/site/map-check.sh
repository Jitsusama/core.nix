# shellcheck shell=bash
# mycelium-map-check: whether the map this machine serves reaches every
# cell of its realm, from the map's own drawing, read in headless Chromium
# (tests/live-reach-test.py). As text it says how many cells the map
# reaches and names each it misses; with --json it prints what the map
# said as one line, {"at","cells","count","failures":[{"cell",
# "orchestration","owner","why"}]}, each why "alone", "unnamed network" or
# "unreachable". It exits 0 when it misses none, 1 when it misses some,
# 69 when the map never read the node and 70 when the check itself broke.
# The browser's own words go to standard error, and a name a peer gave is
# printed as text with its control characters made spaces. Its help and failures take the shape every command of
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

# From its own empty folder, so no Python where it was asked from runs.
cd "${work}" || exit 70
MAP_CHECK_ROOT=${root} MAP_CHECK_SAID=${said} \
  PYTHONPATH=@live_page@ PYTHONDONTWRITEBYTECODE=1 \
  @python@ -m pytest -q -p no:cacheprovider -p live_page --import-mode=importlib \
  --basetemp="${work}/pytest" @test@ > "${work}/pytest.log" 2>&1 || true
cat "${work}/pytest.log" >&2

if [[ ! -s ${said} ]]; then
  if grep -q "the map never read the node" "${work}/pytest.log"; then
    printf '%s\n' "the map never read the mycelium node" \
      "  the page at ${root} drew no live reading in time" \
      "  try  systemctl --user status local-sites, then mycelium machines" >&2
    exit 69
  fi
  printf '%s\n' "the check broke before the map said its reach" \
    "  the browser's words above say where" \
    "  try  mycelium-map-check again, and tell the map's owner if it repeats" >&2
  exit 70
fi
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
