# shellcheck shell=bash
# Builds one live site (site.nix says why it's built this way): typechecks
# its TypeScript on the kit at its strictest, runs its tests if it has any,
# compiles it to the browser's modules, and lays it out with its page, its
# twins and the theme's web kit. Each look's ground and tint fill the page's
# theme-color and its favicon, the two places a site can't name a role. Each
# page gets the strip every one of Joel's sites opens with, written here
# rather than carried by each page, so no app's copy says more than its kind.
# usage: site.sh <tsc> <node> <kit> <site> <name> <web kit> <out>
#   <dark ground> <light ground> <dark tint> <light tint>
set -euo pipefail

tsc=$1 node=$2 kit=$3 site=$4 name=$5 web=$6 out=$7
dark_ground=$8 light_ground=$9 dark_tint=${10} light_tint=${11}

# The sources as they sit in the tree, so the site's imports of the kit
# resolve here as they do there.
work=$(mktemp -d)
mkdir -p "${work}/modules/local-sites" "${work}/modules/${name}"
cp -r "${kit}" "${work}/modules/local-sites/kit"
cp -r "${site}" "${work}/modules/${name}/site"
chmod -R u+w "${work}"
cd "${work}/modules/${name}/site"

"${tsc}" -p tsconfig.json
if [[ -d tests ]]; then
  "${node}" --test --test-reporter=spec tests/*.test.ts
fi
"${tsc}" -p tsconfig.json --noEmit false --rootDir ../.. --outDir "${work}/emitted"

mkdir -p "${out}"
cp "${work}/emitted/${name}/site/"*.js "${out}/"
# The files the browser reads as they are: the page, its styles, its icon,
# any script written by hand rather than in TypeScript, and its Markdown and
# plain-text twins. A README.md is the folder's note for whoever reads the
# tree, not a page, so it stays behind.
find . -maxdepth 1 -type f \( -name '*.html' -o -name '*.css' -o -name '*.svg' \
  -o -name '*.js' -o -name '*.md' -o -name '*.txt' \) -not -name README.md \
  -exec cp -t "${out}/" {} +
for page in "${out}"/*.html "${out}"/*.svg; do
  [[ -f ${page} ]] || continue
  sed -i -e "s|@dark-ground@|${dark_ground}|g" -e "s|@light-ground@|${light_ground}|g" \
    -e "s|@dark-tint@|${dark_tint}|g" -e "s|@light-tint@|${light_tint}|g" "${page}"
done

# The strip, in the block the home writes into the pages it may change, so
# both read alike and strip.css styles both. It says only that the page is
# an app: the page's own line says how live it is, and a word here that read
# as how it stands would contradict that line when the feed broke. It goes
# straight after the skip link, so the skip link stays the first thing a Tab
# reaches. A site is served one folder deep (site.nix says why), so home is
# one folder up.
strip="${work}/strip.html"
cat >"${strip}" <<'STRIP'

<!-- sites strip: localSite writes this -->
<link rel="stylesheet" href="../strip.css">
<nav class="sites-strip" aria-label="Your sites"><a href="../">All your sites</a>
<span>App</span></nav>
<!-- /sites strip -->
STRIP
for page in "${out}"/*.html; do
  [[ -f ${page} ]] || continue
  if grep -q 'sites-strip' "${page}"; then
    echo "${page##*/} carries its own strip; localSite writes it, so take it out" >&2
    exit 1
  fi
  skips=$(grep -c '<a class="skip"' "${page}" || true)
  if [[ ${skips} != 1 ]]; then
    echo "${page##*/} has ${skips} skip links, and the strip goes after its one" >&2
    exit 1
  fi
  sed -i -e "/<a class=\"skip\"/r ${strip}" "${page}"
done
cp -r "${web}" "${out}/kit"
chmod -R u+w "${out}/kit"
