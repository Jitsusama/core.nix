# shellcheck shell=bash
# Builds one live site (site.nix says why it's built this way): typechecks
# its TypeScript on the kit at its strictest, runs its tests if it has any,
# compiles it to the browser's modules, and lays it out with its page and
# its twins. The theme's web kit isn't copied in: every live site reads the
# one copy the sites share, at ../local-sites/web/, so a page that doesn't,
# or that still reads a kit/ of its own, is refused. Each look's ground and tint fill the page's
# theme-color and its favicon, the two places a site can't name a role. Each
# page gets the strip every one of Joel's sites opens with, written here
# rather than carried by each page, so no app's copy says more than its kind.
# usage: site.sh <tsc> <node> <esbuild> <bare-sheet.sh> <kept-comments.sh> <kit>
#   <site> <name> <out> <dark ground> <light ground> <dark tint> <light tint>
set -euo pipefail

tsc=$1 node=$2 esbuild=$3 bare_sheet=$4 kept_comments=$5 kit=$6 site=$7 name=$8 out=$9
dark_ground=${10} light_ground=${11} dark_tint=${12} light_tint=${13}

# The name is written into paths and a config below, so it's held to what
# needs no quoting in either.
[[ ${name} =~ ^[a-z0-9-]+$ ]] || {
  echo "a site's name is lowercase letters, digits and dashes, not ${name}" >&2
  exit 1
}

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
# The files the browser reads as they are: the page, its styles and any
# script written by hand rather than in TypeScript (both laid over below
# without their comments), its icon, and its Markdown and plain-text twins. A README.md
# is the folder's note for whoever reads the tree, not a page, so it stays
# behind.
find . -maxdepth 1 -type f \( -name '*.html' -o -name '*.css' -o -name '*.svg' \
  -o -name '*.js' -o -name '*.md' -o -name '*.txt' \) -not -name README.md \
  -exec cp -t "${out}/" {} +
# A script written by hand goes through the same compiler, so it reaches the
# browser without its comments as the TypeScript does. The compiler is the
# tokenizer because a pattern can't tell a comment from a string, a template
# or a regular expression holding two slashes. It reads the script alone,
# neither checking it nor following its imports, which name TypeScript
# siblings, and keeps its syntax as written. Its own config, since the
# compiler refuses files named beside the site's tsconfig.json. A glob rather
# than compgen, which the builder's bash leaves out, and which then said no
# script was there.
hand_written=(./*.js)
if [[ -e ${hand_written[0]} ]]; then
  cat > "${work}/plain.json" <<EOF
{
  "compilerOptions": {
    "allowJs": true,
    "removeComments": true,
    "noCheck": true,
    "noResolve": true,
    "target": "esnext",
    "module": "preserve",
    "rootDir": "modules/${name}/site",
    "outDir": "plain"
  },
  "include": ["modules/${name}/site/*.js"]
}
EOF
  "${tsc}" -p "${work}/plain.json"
  cp "${work}/plain/"*.js "${out}/"
fi
# The compiler keeps a comment marked to stay however it's told to drop the
# rest, so a script still holding one fails (kept-comments.sh says why).
bash "${kept_comments}" "${out}"/*.js
# The styles go through a CSS compiler for the same reason, as the theme's
# own do (bare-sheet.sh says how, and what it refuses).
for sheet in "${out}"/*.css; do
  [[ -f ${sheet} ]] || continue
  bash "${bare_sheet}" "${esbuild}" "${sheet}" "${sheet}"
done
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
# A page or its styles naming a kit/ that doesn't go through local-sites/,
# however it's spelled: quoted either way or not at all, in a url(), from
# the root, here or a folder up, in any case.
own_kit='(^|[^-a-z0-9_/.])(/|(\./|\.\./)*)kit/'
theme='<link rel="stylesheet" href="../local-sites/web/theme.css">'
cat >"${strip}" <<'STRIP'

<!-- sites strip: localSite writes this -->
<link rel="stylesheet" href="../strip.css">
<nav class="sites-strip" aria-label="Your sites"><a href="../">All your sites</a>
<span>App</span></nav>
<!-- /sites strip -->
STRIP
for file in "${out}"/*.html "${out}"/*.css; do
  [[ -f ${file} ]] || continue
  if grep -qiE "${own_kit}" "${file}"; then
    echo "${file##*/} reads its own kit/; a live site reads ../local-sites/web/" >&2
    exit 1
  fi
done
for page in "${out}"/*.html; do
  [[ -f ${page} ]] || continue
  if grep -q 'sites-strip' "${page}"; then
    echo "${page##*/} carries its own strip; localSite writes it, so take it out" >&2
    exit 1
  fi
  if ! grep -qF "${theme}" "${page}"; then
    echo "${page##*/} doesn't read the theme from the sites' one copy: ${theme}" >&2
    exit 1
  fi
  # A face the first screen sets, not asked for ahead, is found only once the
  # styles are read, so the words paint late or jump as it lands: three pages
  # did on 22 November. These are the theme's web kit's own file names
  # (modules/theme/web/web.nix); its italic mono sets nothing on a first
  # screen, so it's left to load when asked. The whole tag is asked for:
  # without crossorigin a preload is never matched to the face's own fetch,
  # and the page is read without its comments, where a preload asks for
  # nothing. Each ends as HTML ends one: at once as <!--> or <!--->, or else
  # at its first two dashes or more and a >, or those and !>, dashes and a
  # lone -> inside it held; and one never ended runs to the page's end. The
  # at-once forms count only where a comment starts, so <!--> inside one ends
  # it; its body can't open with > or ->, so sed's longest match can't read
  # <!--> x --> as one comment.
  first='\([^->]\|-[^->]\|--\+[^->!]\|--\+![^>]\)'
  body='\([^-]\|-[^->]\|->\|--\+[^->!]\|--\+![^>]\)'
  ended="s/<!--\(-\?>\|\(${first}${body}*\)\?--\+!\?>\)//g"
  bare=$(tr '\n' ' ' <"${page}" | sed -e "${ended}" -e 's/<!--.*$//')
  for face in fraunces fraunces-italic monaspace; do
    preload="<link rel=\"preload\" href=\"../local-sites/web/fonts/${face}.woff2\" as=\"font\""
    preload+=" type=\"font/woff2\" crossorigin>"
    if [[ ${bare} != *"${preload}"* ]]; then
      echo "${page##*/} doesn't preload ${face}.woff2, so its words paint late: ${preload}" >&2
      exit 1
    fi
  done
  skips=$(grep -c '<a class="skip"' "${page}" || true)
  if [[ ${skips} != 1 ]]; then
    echo "${page##*/} has ${skips} skip links, and the strip goes after its one" >&2
    exit 1
  fi
  sed -i -e "/<a class=\"skip\"/r ${strip}" "${page}"
done
