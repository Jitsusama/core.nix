# shellcheck shell=bash
# mycelium-map-check: whether the map this machine serves reaches every
# cell of its realm, from the map's own drawing, read in headless Chromium
# (tests/live-reach-test.py). It prints what the map said as one line of
# JSON, {"at","cells","count","failures":[{"cell","orchestration","owner",
# "why"}]}, each why "alone", "unnamed network" or "unreachable", and exits
# 0 when it misses none, 1 when it misses some, and 2 when the map never
# read the node. The browser's own words go to standard error.
#
# MAP_CHECK_ROOT, the sites' root, defaults to this machine's.
# Given by Nix: @python@, @test@ and @live_page@.

work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
said=${work}/said.json

MAP_CHECK_ROOT=${MAP_CHECK_ROOT:-http://sites.localhost:8484/} MAP_CHECK_SAID=${said} \
  PYTHONPATH=@live_page@ PYTHONDONTWRITEBYTECODE=1 \
  @python@ -m pytest -q -p no:cacheprovider -p live_page --import-mode=importlib \
  --basetemp="${work}/pytest" @test@ >&2 || true

if [[ ! -s ${said} ]]; then
  echo "mycelium-map-check: the map never read the node" >&2
  exit 2
fi
jq -c . "${said}"
[[ $(jq .count "${said}") == 0 ]]
