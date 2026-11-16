# shellcheck shell=bash
# Checks the home's builder against a sites.json of three rows: a growing
# site with a strip and a page of its own kind, a growing one with none, and
# a settled one. The home lists the two growing ones in order and the
# settled one under Kept for Reference, with every name escaped; llms.txt
# and the twin list all three; each stripped page gets its strip after the
# skip link, linking home from its depth; the home's own files are put
# beside it; its theme-color takes each look's ground from the kit, and
# names none without one; a second run changes no file; and a root with no
# sites.json is left alone.
# usage: home-builds-from-its-rows.sh <python> <sources>
set -euo pipefail

python=$1
sources=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
root=$work/sites

fail() {
  echo "home-builds-from-its-rows: $*" >&2
  exit 1
}

mkdir -p "$work/empty"
"$python" "$sources/home.py" "$sources" "$work/empty" > /dev/null
left=$(ls -A "$work/empty")
[ -z "$left" ] || fail "a root with no sites.json gained files"

mkdir -p "$root/map" "$root/notes" "$root/old" "$root/kit"
# The kit's grounds, as theme.css writes them, under stand-in names: the
# home's two theme-color metas take them, one per look.
cat > "$root/kit/theme.css" << 'EOF'
:root,
:root[data-theme="dark"] {
  --accent: jade;
  --background: forest;
}
@media (prefers-color-scheme: light) {
  :root:not([data-theme="dark"]) {
  --background: paper;
  }
}
EOF
cat > "$root/sites.json" << 'EOF'
{"about": "a stand-in", "sites": [
  {"path": "map/", "name": "The map", "what": "Where things are.", "asks": "Say what's missing.",
   "when": "15 Nov", "kind": "app", "status": "growing", "strip": true,
   "pages": {"tree.html": "test"}},
  {"path": "notes/", "name": "Notes & such", "what": "Notes.", "asks": "",
   "when": "14 Nov", "kind": "record", "status": "growing"},
  {"path": "old/", "name": "The old one", "what": "Kept.", "asks": "",
   "when": "11 Nov", "kind": "mock", "status": "settled", "strip": true}
]}
EOF
for page in map/index.html map/tree.html old/index.html; do
  printf '<!doctype html>\n<body>\n<a class="skip" href="#main">Skip</a>\n<main id="main"></main>\n' \
    > "$root/$page"
done

said=$("$python" "$sources/home.py" "$sources" "$root")
[ "$said" = "home: 2 sites, 1 kept; strips on 3 pages" ] || fail "it said: $said"

for file in home.js home.css theme.js strip.css; do
  cmp -s "$sources/$file" "$root/$file" || fail "$file wasn't put beside the home"
done

index=$root/index.html
grep -q '<span class="name">The map</span>' "$index" || fail "the home lacks The map"
grep -q '<span class="name">Notes &amp; such</span>' "$index" || fail "a name wasn't escaped"
map_at=$(grep -n 'The map' "$index" | cut -d: -f1)
notes_at=$(grep -n 'Notes &amp; such' "$index" | cut -d: -f1)
shelf_at=$(grep -n 'Kept for Reference</h2>' "$index" | cut -d: -f1)
old_at=$(grep -n 'The old one' "$index" | cut -d: -f1)
[ "$map_at" -lt "$notes_at" ] && [ "$notes_at" -lt "$shelf_at" ] && [ "$shelf_at" -lt "$old_at" ] ||
  fail "the home's rows are out of order: map $map_at, notes $notes_at, shelf $shelf_at, old $old_at"
grep -q 'Mock · settled' "$index" || fail "the settled row doesn't say so"
grep -q 'media="(prefers-color-scheme: dark)" content="forest"' "$index" ||
  fail "the dark theme-color isn't the kit's dark ground"
grep -q 'media="(prefers-color-scheme: light)" content="paper"' "$index" ||
  fail "the light theme-color isn't the kit's light ground"

listed=$(grep -c '^- \[' "$root/llms.txt")
[ "$listed" = 3 ] || fail "llms.txt doesn't list all three"
grep -q '^- \[The map\](map/llms.txt): app, 15 Nov' "$root/llms.txt" || fail "llms.txt's map line"
listed=$(grep -c '^- \[' "$root/index.md")
[ "$listed" = 3 ] || fail "the twin doesn't list all three"

grep -A2 'class="skip"' "$root/map/index.html" | grep -q 'sites strip' ||
  fail "the map's strip isn't after its skip link"
grep -q 'href="../">All your sites</a> <span>App</span>' "$root/map/index.html" ||
  fail "the map's strip doesn't link home or say App"
grep -q 'Asks: Say what&#x27;s missing.' "$root/map/index.html" || fail "the map's strip lacks its ask"
grep -q '<span>Test</span>' "$root/map/tree.html" || fail "the map's own page doesn't say Test"
grep -q '<span>settled</span>' "$root/old/index.html" || fail "the old one's strip doesn't say settled"

before=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
"$python" "$sources/home.py" "$sources" "$root" > /dev/null
after=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
[ "$before" = "$after" ] || fail "a second run changed a file"
strips=$(grep -c 'sites strip: home.py' "$root/map/index.html")
[ "$strips" = 1 ] || fail "a strip was doubled"

# Without the kit, the home names no ground rather than a made-up one.
rm "$root/kit/theme.css"
"$python" "$sources/home.py" "$sources" "$root" > /dev/null
if grep -q 'theme-color' "$index"; then
  fail "the home names a ground with no kit to take it from"
fi
