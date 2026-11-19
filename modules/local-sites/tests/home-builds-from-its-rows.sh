# shellcheck shell=bash
# Checks the home's builder against a sites.json of three rows: a growing site
# with a strip and a page of its own kind, a growing one with none, and a
# settled one. The home lists the two growing ones in their beds in order
# (live before on record) and the settled one under Kept, with every name
# escaped, a contents line naming each bed and a standing line counting them;
# llms.txt and the twin list all three; each stripped page gets its strip
# after the skip link, linking home from its depth; the home's own files are
# put beside it, its mark drawn in each look's tint, and its old scripts taken
# away, while its page loads the kit's script and switch and the theme and its
# fonts from the sites' shared copy, never a kit/ of the documents'; its
# theme-color and mark take each look's ground and tint from the theme it's
# handed, not a kit/ among the documents, and names none without one, saying
# so; a theme that isn't a stylesheet fails the build; a look with no tint
# takes the other's, and a tint or ground that isn't a colour is left out and
# said, never written in; a second run changes no file; a broken record keeps
# the last build; a root with no sites.json is left alone; a row Nix declares
# stands in for the record's, or goes first, and rows.json says every row
# listed as it was written, in the home's order, but for a recorded row's
# claim on a kind a declared row is home to, left out and said; a recorded
# row whose path leads off the sites is left out and said, and a declared one
# fails the build; and a document loading a script file of its own is named, on any of
# its pages, however its tag is spelled and even past a byte that isn't UTF-8,
# while one loading only the kit's, and an app loading its own, are not.
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
"$python" "$sources/home.py" "$sources" "$work/empty" "$work/empty/kit/theme.css" > /dev/null
left=$(ls -A "$work/empty")
[ -z "$left" ] || fail "a root with no sites.json gained files"

mkdir -p "$root/map" "$root/notes" "$root/old" "$root/kit"
# The kit's grounds, as theme.css writes them, under stand-in names: the
# home's two theme-color metas take them, one per look.
cat > "$root/kit/theme.css" << 'EOF'
:root,
:root[data-theme="dark"] {
  --action: amber;
  --background: forest;
}
@media (prefers-color-scheme: light) {
  :root:not([data-theme="dark"]) {
  --action: ochre;
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
   "when": "14 Nov", "kind": "record", "status": "growing",
   "pages": {"more.html": "record"}},
  {"path": "old/", "name": "The old one", "what": "Kept.", "asks": "",
   "when": "11 Nov", "kind": "mock", "status": "settled", "strip": true}
]}
EOF
for page in map/index.html map/tree.html old/index.html notes/index.html notes/more.html; do
  printf '<!doctype html>\n<body>\n<a class="skip" href="#main">Skip</a>\n<main id="main"></main>\n' \
    > "$root/$page"
done
script() { printf '<script %s></script>\n' "$1" >> "$root/$2"; }
script 'src="site.js"' map/index.html
script 'type="module" src="/local-sites/kit/document.js"' notes/index.html
script 'src="site.js" defer' old/index.html
script 'type="module" src="ways.js"' old/index.html
printf '\xff\n' >> "$root/notes/more.html"
script "type=module SRC='/local-sites/kit/../x.js'" notes/more.html
script "src=/local-sites/kit/../x.js" notes/index.html

# What an older builder copied in beside the home, which this one takes away.
touch "$root/home.js" "$root/theme.js"

said=$("$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css")
owning="scripts of their own in notes/ (/local-sites/kit/../x.js), old/ (site.js, ways.js)"
[ "$said" = "home: 2 sites, 1 kept; strips on 3 pages; $owning" ] || fail "it said: $said"

for file in home.css strip.css; do
  cmp -s "$sources/$file" "$root/$file" || fail "$file wasn't put beside the home"
done
for file in home.js theme.js; do
  [ ! -e "$root/$file" ] || fail "the old $file was left beside the home"
done

index=$root/index.html
grep -q '<script type="module" src="/local-sites/kit/home.js"></script>' "$index" ||
  fail "the home doesn't load the kit's home.js"
grep -q '<link rel="stylesheet" href="/local-sites/kit/switch.css">' "$index" ||
  fail "the home doesn't load the kit's switch.css"
# The theme and its fonts come from the one copy the sites share, so a visit
# from the home to a site fetches them once.
for part in theme.css base.css fonts/fraunces.woff2; do
  grep -q "href=\"/local-sites/web/$part\"" "$index" ||
    fail "the home doesn't load $part from the sites' shared copy"
done
# However it's spelled, as site.sh refuses it on a live page.
if grep -qiE '(^|[^-a-z0-9_/.])(/|(\./|\.\./)*)kit/' "$index"; then
  fail "the home still loads a kit/ of the documents'"
fi
! grep -qE 'src="(home|theme)\.js"' "$index" || fail "the home still loads a script of its own"
grep -q '<span class="name">The map</span>' "$index" || fail "the home lacks The map"
grep -q '<span class="name">Notes &amp; such</span>' "$index" || fail "a name wasn't escaped"
map_at=$(grep -n 'The map' "$index" | cut -d: -f1)
notes_at=$(grep -n 'Notes &amp; such' "$index" | cut -d: -f1)
shelf_at=$(grep -n '<h2 id="kept">' "$index" | cut -d: -f1)
old_at=$(grep -n 'The old one' "$index" | cut -d: -f1)
[ "$map_at" -lt "$notes_at" ] && [ "$notes_at" -lt "$shelf_at" ] && [ "$shelf_at" -lt "$old_at" ] ||
  fail "the home's rows are out of order: map $map_at, notes $notes_at, shelf $shelf_at, old $old_at"
grep -q 'Mock · settled' "$index" || fail "the settled row doesn't say so"
for bed in live on-record kept; do
  grep -q "<a href=\"#$bed\">" "$index" || fail "the contents line lacks the $bed bed"
done
if grep -q 'href="#explained"' "$index"; then
  fail "the contents line names a bed with nothing in it"
fi
grep -q '2 growing <span class="sep">·</span> 1 asking something of you' "$index" ||
  fail "the standing line doesn't count the sites"
if grep -q '@standing@\|@contents@' "$index"; then
  fail "a placeholder was left in the home"
fi
grep -q 'media="(prefers-color-scheme: dark)" content="forest"' "$index" ||
  fail "the dark theme-color isn't the kit's dark ground"
grep -q 'media="(prefers-color-scheme: light)" content="paper"' "$index" ||
  fail "the light theme-color isn't the kit's light ground"
# The mark takes each look's tint.
mark=$root/favicon.svg
grep -q 'dark) { g { stroke: amber; }' "$mark" || fail "the dark mark isn't the kit's tint"
grep -q '^    g { stroke: ochre; }' "$mark" || fail "the light mark isn't the kit's tint"
# Builds the home beside a kit whose theme.css is $2, in $work/$1, keeping
# what it said on stderr in $work/$1.said.
kit_of() {
  mkdir -p "$work/$1/kit"
  cp "$root/sites.json" "$work/$1/"
  printf '%s\n' "$2" > "$work/$1/kit/theme.css"
  "$python" "$sources/home.py" "$sources" "$work/$1" "$work/$1/kit/theme.css" \
    > /dev/null 2> "$work/$1.said"
}
# Whether the mark built in $work/$1 strokes $2 in the dark look and $3 in the
# light one, failing with $4 otherwise.
strokes() {
  if ! grep -q "dark) { g { stroke: $2; }" "$work/$1/favicon.svg" ||
    ! grep -q "^    g { stroke: $3; }" "$work/$1/favicon.svg"; then
    fail "$4"
  fi
}
light="@media (prefers-color-scheme: light) { :root { --action: ochre; } }"
# A look the kit names no tint for takes the other look's.
kit_of dark-only ':root { --action: amber; }'
strokes dark-only amber amber "the light mark didn't take the dark look's tint"
kit_of light-only "$light"
strokes light-only ochre ochre "the dark mark didn't take the light look's tint"
# A colour with slash alpha, under a stand-in function's name, is a colour.
kit_of alpha ':root { --action: tide(1 2 3 / 50%); }'
strokes alpha 'tide(1 2 3 / 50%)' 'tide(1 2 3 / 50%)' "the mark refused a slash alpha"
# What would close the mark's style, start another rule, open a comment or
# read a property a favicon can't is left out, and said; with no other tint
# the mark draws in currentColor.
for odd in 'red}</style><script>x()</script>' 'red } g { stroke: blue' 'red /* x' 'var(--gold)'; do
  kit_of odd ":root { --action: $odd; }"
  ! grep -q '<script\|blue\|/\*\|var(' "$work/odd/favicon.svg" ||
    fail "the mark took $odd for a tint"
  strokes odd currentColor currentColor "the mark without a tint has no ink"
  told=$(< "$work/odd.said")
  [[ $told == *"the dark look's tint, '$odd', is no colour"* ]] ||
    fail "a tint left out wasn't said: $told"
done
# A ground that would break out of the page's theme-color is left out of it.
kit_of ground ':root { --background: red"><script>y()</script>; }'
! grep -q '<script>y' "$work/ground/index.html" || fail "the home took markup for a ground"

listed=$(grep -c '^- \[' "$root/llms.txt")
[ "$listed" = 3 ] || fail "llms.txt doesn't list all three"
grep -q '^- \[The map\](map/llms.txt): app, 15 Nov' "$root/llms.txt" || fail "llms.txt's map line"
listed=$(grep -c '^- \[' "$root/index.md")
[ "$listed" = 3 ] || fail "the twin doesn't list all three"

grep -A2 'class="skip"' "$root/map/index.html" | grep -q 'sites strip' ||
  fail "the map's strip isn't after its skip link"
grep -q 'href="../">All your sites</a> <span>App</span>' "$root/map/index.html" ||
  fail "the map's strip doesn't link home or say App"
grep -q '<span>App</span> <span>built ' "$root/map/index.html" ||
  fail "the app's strip doesn't say when it was built"
grep -q '<span>as of ' "$root/map/index.html" && fail "the app's strip reads as of, like a record"
grep -q 'Asks: Say what&#x27;s missing.' "$root/map/index.html" || fail "the map's strip lacks its ask"
grep -q '<span>Test</span>' "$root/map/tree.html" || fail "the map's own page doesn't say Test"
grep -q '<span>settled</span>' "$root/old/index.html" || fail "the old one's strip doesn't say settled"

before=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
"$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css" > /dev/null
after=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
[ "$before" = "$after" ] || fail "a second run changed a file"
strips=$(grep -c 'sites strip: home.py' "$root/map/index.html")
[ "$strips" = 1 ] || fail "a strip was doubled"

# The rows Nix declares stand in for the record's at the same path, in its
# place, and go first when the record has none. llms.txt lists every row in
# order, whatever its bed, so it shows which.
cat > "$work/declared.json" << 'EOF'
{"sites": [
  {"path": "grove/", "name": "The grove", "what": "Trees.", "kind": "app", "when": "live"},
  {"path": "old/", "name": "The old one, declared", "what": "Live.", "kind": "app", "when": "live",
   "homes": ["tree", {"kind": "heron", "address": "#heron={name}"}]}
]}
EOF
said=$("$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css" "$work/declared.json")
# Declared, the old one is an app, whose scripts are its own business.
owning="scripts of their own in notes/ (/local-sites/kit/../x.js)"
[ "$said" = "home: 4 sites, 0 kept; strips on 2 pages; $owning" ] || fail "declared, it said: $said"
grep -q '<span class="name">The old one</span>' "$index" &&
  fail "the record's old row outlived its declaration"
listed=$(grep -o '^- \[[^]]*\]' "$root/llms.txt" | tr '\n' ' ')
[ "$listed" = "- [The grove] - [The map] - [Notes & such] - [The old one, declared] " ] ||
  fail "the declared rows aren't first and in place: $listed"
# What reads each site's homes reads them in rows.json, each row whole.
[ -f "$root/rows.json" ] || fail "the home wrote no rows.json"
# Each is the row as written, the declared one where Nix declares its path.
rows=$("$python" -c 'import json, sys
listed = json.load(open(sys.argv[1]))["sites"]
written = {row["path"]: row for name in sys.argv[2:] for row in json.load(open(name))["sites"]}
whole = all(row == written.get(row["path"]) for row in listed)
print(*(row["path"] for row in listed), "whole" if whole else "rewritten")' \
  "$root/rows.json" "$root/sites.json" "$work/declared.json")
[ "$rows" = "grove/ map/ notes/ old/ whole" ] || fail "rows.json isn't every row as written: $rows"
before=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
"$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css" "$work/declared.json" \
  > /dev/null
after=$(cd "$root" && find . -type f -printf '%p %T@\n' | sort)
[ "$before" = "$after" ] || fail "a second run with declared rows changed a file"
"$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css" > /dev/null

# A kind a declared row is home to stays its home: a recorded row claiming it
# by name or by pattern loses that claim in rows.json, and it's said, while
# its other homes, and a declared row's own, stand.
mkdir -p "$work/claimed"
cat > "$work/claimed/sites.json" << 'EOF'
{"sites": [
  {"path": "notes/", "name": "Notes", "kind": "record",
   "homes": ["tree", {"kind": "heron", "address": "#heron={name}"}, "otter"]},
  {"path": "./shed/", "name": "The shed", "kind": "mock", "homes": ["heron"]},
  {"path": "pond/", "name": "The pond", "kind": "mock", "homes": ["otter"]}
]}
EOF
said=$("$python" "$sources/home.py" "$sources" "$work/claimed" "$work/claimed/kit/theme.css" \
  "$work/declared.json")
taken="homes left to the declared sites: notes/'s tree, notes/'s heron, ./shed/'s heron"
[ "$said" = "home: 5 sites, 0 kept; strips on 0 pages; $taken" ] ||
  fail "with recorded rows claiming declared homes, it said: $said"
homes=$("$python" -c 'import json, sys
rows = json.load(open(sys.argv[1]))["sites"]
print(json.dumps({row["path"]: row.get("homes") for row in rows}))' \
  "$work/claimed/rows.json")
want='{"grove/": null, "old/": ["tree", {"kind": "heron", "address": "#heron={name}"}],'
want+=' "notes/": ["otter"], "./shed/": [], "pond/": ["otter"]}'
[ "$homes" = "$want" ] || fail "rows.json didn't leave declared kinds to their sites: $homes"

# A path the record holds twice, however spelled, is listed once when
# declared, and with no record or folder yet the declared rows make both.
mkdir -p "$work/twice"
grove=$(sed -n '2s/},$/}/p' "$work/declared.json")
printf '{"sites": [%s, %s]}\n' "${grove/grove\//grove}" "${grove/grove\//.\/grove\/}" \
  > "$work/twice/sites.json"
"$python" "$sources/home.py" "$sources" "$work/twice" "$work/twice/kit/theme.css" \
  "$work/declared.json" > /dev/null
listed=$(grep -c '^- \[The grove\]' "$work/twice/llms.txt")
[ "$listed" = 1 ] || fail "a twice-recorded declared site was listed $listed times"
"$python" "$sources/home.py" "$sources" "$work/fresh/sites" "$work/fresh/sites/kit/theme.css" \
  "$work/declared.json" > /dev/null
grep -q '<span class="name">The grove</span>' "$work/fresh/sites/index.html" ||
  fail "with no record, the declared rows made no home"
# A row whose path leads anywhere but a page of the sites' own is left out of
# every list, and said, while the rest are built, however a browser would
# read it: a leading space or a tab inside is dropped before the scheme is.
mkdir -p "$work/strays"
cat > "$work/strays/sites.json" << 'EOF'
{"sites": [
  {"path": "javascript:x()", "name": "Stray", "kind": "mock"},
  {"path": "//gull.example/", "name": "Stray", "kind": "mock"},
  {"path": "../up/", "name": "Stray", "kind": "mock"},
  {"path": "a\\b/", "name": "Stray", "kind": "mock"},
  {"path": " javascript:x()", "name": "Stray", "kind": "mock"},
  {"path": "java\tscript:x()", "name": "Stray", "kind": "mock"},
  {"path": "", "name": "Stray", "kind": "mock"},
  {"path": "/root/", "name": "Stray", "kind": "mock"},
  {"path": "./.dot/", "name": "Stray", "kind": "mock"},
  {"path": "\u0663/", "name": "Stray", "kind": "mock"},
  {"path": 5, "name": "Stray", "kind": "mock"},
  {"path": "tern/.hidden/", "name": "Stray", "kind": "mock"},
  {"path": "tern/javascript:x()", "name": "Stray", "kind": "mock"},
  {"path": "./heron/", "name": "Heron", "kind": "mock"},
  {"path": "tern/", "name": "Tern", "kind": "mock"}
]}
EOF
said=$("$python" "$sources/home.py" "$sources" "$work/strays" "$work/strays/kit/theme.css")
left="left out 'javascript:x()', '//gull.example/', '../up/', 'a\\\\b/', ' javascript:x()',"
left+=" 'java\\tscript:x()', '', '/root/', './.dot/', '٣/', 5,"
left+=" 'tern/.hidden/', 'tern/javascript:x()', no path of ours"
[[ $said == *"; $left" ]] || fail "the strays weren't said: $said"
for listing in index.html index.md llms.txt rows.json; do
  [ -f "$work/strays/$listing" ] || fail "the strays left no $listing"
  ! grep -q 'Stray' "$work/strays/$listing" || fail "a stray row was listed in $listing"
done
for kept in Tern Heron; do
  grep -q "<span class=\"name\">$kept</span>" "$work/strays/index.html" ||
    fail "the strays kept $kept out too"
done
# NaN, an infinity however it's spelled, and a lone surrogate are nothing a
# strict reader of rows.json takes, nor the page draws, so the record isn't
# read and the last build stays.
mkdir -p "$work/unread"
for field in '"weight": NaN' '"weight": 1e999' '"note": "\ud800"'; do
  echo "{\"sites\": [{\"path\": \"gull/\", \"name\": \"Gull\", \"kind\": \"mock\", $field}]}" \
    > "$work/unread/sites.json"
  said=$("$python" "$sources/home.py" "$sources" "$work/unread" "$work/unread/kit/theme.css")
  [[ $said == *"can't be read"*"so the last build stays" ]] || fail "$field was read: $said"
  [ ! -e "$work/unread/index.html" ] || fail "$field was built"
done
# A stray path Nix declared is the build's fault, so it fails rather than
# leaving the site out.
mkdir -p "$work/astray"
echo '{"sites": [{"path": "javascript:x()", "name": "Stray", "kind": "app"}]}' \
  > "$work/astray.json"
if "$python" "$sources/home.py" "$sources" "$work/astray" "$work/astray/kit/theme.css" \
  "$work/astray.json" > /dev/null 2> "$work/astray.said"; then
  fail "a stray declared row didn't fail the build"
fi
told=$(< "$work/astray.said")
[[ $told == *"declared rows 'javascript:x()' lead off the sites"* ]] ||
  fail "a stray declared row failed for another reason: $told"
written=$(ls -A "$work/astray")
[ -z "$written" ] || fail "a stray declared row failed after writing $written"

# The theme the home is handed is the one it wears, not a kit/ among the
# documents, which a session can write and the home no longer reads.
mkdir -p "$work/handed"
sed -e 's/forest/moss/' -e 's/paper/linen/' -e 's/amber/rust/' -e 's/ochre/sand/' \
  "$root/kit/theme.css" > "$work/handed/theme.css"
"$python" "$sources/home.py" "$sources" "$root" "$work/handed/theme.css" > /dev/null
grep -q 'media="(prefers-color-scheme: dark)" content="moss"' "$index" ||
  fail "the home took its dark ground from somewhere other than the theme it was handed"
grep -q 'media="(prefers-color-scheme: light)" content="linen"' "$index" ||
  fail "the home took its light ground from somewhere other than the theme it was handed"
grep -q 'dark) { g { stroke: rust; }' "$root/favicon.svg" ||
  fail "the home's mark took its dark tint from somewhere other than the theme it was handed"
grep -q '^    g { stroke: sand; }' "$root/favicon.svg" ||
  fail "the home's mark took its light tint from somewhere other than the theme it was handed"
# A caller still on the old shape, handing the declared rows where the theme
# goes, fails rather than building a home that names no look.
if "$python" "$sources/home.py" "$sources" "$root" "$work/declared.json" \
  > /dev/null 2> "$work/shape.said"; then
  fail "the declared rows handed as the theme built anyway"
fi
told=$(< "$work/shape.said")
[[ $told == *"isn't a stylesheet"* ]] ||
  fail "the declared rows as the theme failed for another reason: $told"

# Without the kit, the home names no ground rather than a made-up one.
rm "$root/kit/theme.css"
"$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css" > /dev/null \
  2> "$work/gone.said"
if grep -q 'theme-color' "$index"; then
  fail "the home names a ground with no kit to take it from"
fi
grep -q "the theme $root/kit/theme.css isn't there" "$work/gone.said" ||
  fail "a missing theme went unsaid"

# A record caught half written, or broken by a slip, keeps the last good
# build and says why, rather than failing and leaving the home to it.
cp "$index" "$work/last.html"
head -c 200 "$root/sites.json" > "$work/cut.json"
mv "$work/cut.json" "$root/sites.json"
said=$("$python" "$sources/home.py" "$sources" "$root" "$root/kit/theme.css") ||
  fail "a broken record failed the build"
case $said in
  "home: sites.json can't be read ("*"), so the last build stays") ;;
  *) fail "a broken record said: $said" ;;
esac
cmp -s "$work/last.html" "$index" || fail "a broken record changed the home"
