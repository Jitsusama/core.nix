# shellcheck shell=bash
# Checks that localSite writes the strip every live page opens with: once,
# straight after the skip link, linking home one folder up and saying only
# that the page is an app, never a word that reads as how it stands. And
# that a page carrying a strip of its own fails its build with why, since
# two strips, or one saying more than the kind, is what this replaces. And
# that the page reads the theme's web kit from the one copy the sites share,
# with none of its own, and a page still reading its own kit/ fails too.
# And that the sheets a site, the kit and the theme build reach a page
# without their comments, and its scripts without one marked to stay; and a
# sheet esbuild refuses, warns on or crashes on, or a sheet or a script
# keeping a comment, fails, saying which.
# usage: each-live-page-opens-with-the-strip.sh <built site> <site.sh> <tsc>
#   <node> <esbuild> <bare-sheet.sh> <kept-comments.sh> <kit> <site source>
#   <name> <built kit> <web kit>
set -euo pipefail

built=$1 build=$2 tsc=$3 node=$4 esbuild=$5 bare_sheet=$6 kept_comments=$7 kit=$8
source=$9 name=${10} built_kit=${11} web_kit=${12}
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT

page=${built}/index.html
strips=$(grep -c 'sites strip: localSite writes this' "${page}" || true)
[[ ${strips} == 1 ]] || {
  echo "the page carries ${strips} strips, not one" >&2
  exit 1
}
grep -A2 '<a class="skip"' "${page}" | grep -q 'sites strip: localSite' || {
  echo "the strip isn't straight after the skip link" >&2
  exit 1
}
grep -q '<link rel="stylesheet" href="../strip.css">' "${page}" || {
  echo "the strip doesn't take its styles from the home, one folder up" >&2
  exit 1
}
home='<nav class="sites-strip" aria-label="Your sites"><a href="../">All your sites</a>'
said=$(grep -A1 '<nav class="sites-strip"' "${page}")
[[ ${said} == "${home}"$'\n''<span>App</span></nav>' ]] || {
  echo "the strip says more than the way home and that the page is an app: ${said}" >&2
  exit 1
}
shared='<link rel="stylesheet" href="../local-sites/web/theme.css">'
grep -qF "${shared}" "${page}" || {
  echo "the page doesn't read the theme from the sites' one copy" >&2
  exit 1
}
[[ ! -e ${built}/kit ]] || {
  echo "the site carries a kit/ of its own" >&2
  exit 1
}

# What every build below is handed but the site.
build_with=("${build}" "${tsc}" "${node}" "${esbuild}" "${bare_sheet}" "${kept_comments}"
  "${kit}")
looks=("${name}" "${work}/out" black white black white)

# A copy of the site's source, at `$1`, for a case to change.
copied() {
  cp -r "${source}" "$1"
  chmod -R u+w "$1"
}

# Builds the site at `$1` and expects it refused, saying `$2`; `$3` names the
# case, and `$4`, if given, is the esbuild to build with.
refused() {
  rm -rf "${work}/out"
  local with=("${build_with[@]}")
  [[ -z ${4-} ]] || with[3]=$4
  if bash "${with[@]}" "$1" "${looks[@]}" 2>"${work}/said"; then
    echo "$3 built anyway" >&2
    exit 1
  fi
  said=$(<"${work}/said")
  [[ ${said} == *"$2"* ]] || {
    echo "the build refused $3 without saying so: ${said}" >&2
    exit 1
  }
}

# A page that brings its own strip is refused, saying which page and why.
copied "${work}/strip"
sed -i '/<a class="skip"/a <nav class="sites-strip"><span>App</span> <span>live</span></nav>' \
  "${work}/strip/index.html"
refused "${work}/strip" 'index.html carries its own strip' "a page with a strip of its own"

# A page still reading its own kit/, as a document does, is refused too, as
# are its styles reading one however they spell it.
copied "${work}/kit"
sed -i 's|href="../local-sites/web/base.css"|href="kit/base.css"|' "${work}/kit/index.html"
refused "${work}/kit" 'index.html reads its own kit/' "a page reading its own kit/"
copied "${work}/styles"
echo "@font-face { src: url('../KIT/fonts/x.woff2'); }" >>"${work}/styles/site.css"
refused "${work}/styles" 'site.css reads its own kit/' "styles reading a kit/ a folder up"

# And a page that doesn't read the theme from the sites' one copy at all.
copied "${work}/themeless"
sed -i '\|href="../local-sites/web/theme.css"|d' "${work}/themeless/index.html"
refused "${work}/themeless" "index.html doesn't read the theme from the sites' one copy" \
  "a page reading no theme"

# And one that leaves a face the first screen sets to be found late.
copied "${work}/late"
sed -i '\|href="../local-sites/web/fonts/monaspace.woff2"|d' "${work}/late/index.html"
refused "${work}/late" "index.html doesn't preload monaspace.woff2" "a page preloading no mono"

# A preload without crossorigin is never matched to the face's own fetch,
# so the face comes twice, late, and one in a comment asks for nothing.
copied "${work}/uncrossed"
sed -i '\|fonts/monaspace.woff2|s| crossorigin>|>|' "${work}/uncrossed/index.html"
refused "${work}/uncrossed" "index.html doesn't preload monaspace.woff2" \
  "a page preloading its mono without crossorigin"
copied "${work}/hushed"
sed -i '\|fonts/monaspace.woff2|{s|^|<!-- |;s|$| -->|}' "${work}/hushed/index.html"
refused "${work}/hushed" "index.html doesn't preload monaspace.woff2" \
  "a page preloading its mono only in a comment"

# The built sheets hold no comment, so each sheet's write in place landed,
# and nor do the kit's sheets or the theme's base.css, so their builds still
# send each through the same script: those builds take no source but their
# own, so a comment arriving in one is what shows it. The kit's scripts are
# checked by the kit's own build.
sheets=("${built}"/*.css)
[[ -e ${sheets[0]} ]] || {
  echo "the built site has no sheet to read" >&2
  exit 1
}
status=0
commented=$(grep -l '/\*' "${sheets[@]}" "${built_kit}/switch.css" "${built_kit}/head.css" \
  "${web_kit}/base.css") ||
  status=$?
((status <= 1)) || {
  echo "a built sheet couldn't be read" >&2
  exit 1
}
[[ -z ${commented} ]] || {
  echo "a built sheet still holds a comment: ${commented}" >&2
  exit 1
}

# bare-sheet.sh on one sheet: a sheet esbuild refuses is refused, saying
# which and what esbuild said, and so is one it only warns on, and one it
# crashes on. Each is told by an esbuild standing in, since which of these
# a given slip is belongs to esbuild's release; each fails only on site.css
# and hands any other to the real esbuild, so a site's other sheets pass.
# Each names this bash, since a build's sandbox has no /usr/bin/env.
stand_in() {
  echo "#!${BASH}"
  echo "real=$(printf %q "${esbuild}")"
  cat <<'EOF'
if [[ ${*: -1} == */site.css ]]; then
EOF
  echo "$1"
  cat <<'EOF'
fi
exec "${real}" "$@"
EOF
}
stand_in '  echo "stand-in: refused" >&2; exit 1' >"${work}/broke"
stand_in '  echo "stand-in: careful" >&2' >"${work}/warns"
stand_in '  echo "stand-in: crashed" >&2; exit 2' >"${work}/crashes"
chmod +x "${work}/broke" "${work}/warns" "${work}/crashes"
mkdir "${work}/sheets" "${work}/bare"
sheet=${work}/sheets/site.css

# Runs bare-sheet.sh on `sheet` with the esbuild `$1` and expects it refused,
# saying `$2` and what esbuild said, `$3`; `$4` names the case. It writes to
# out.css, so the refusal names the sheet by where it was to go, as one read
# from the store must be.
sheet_refused() {
  if bash "${bare_sheet}" "$1" "${sheet}" "${work}/bare/out.css" 2>"${work}/said"; then
    echo "$4 was written anyway" >&2
    exit 1
  fi
  said=$(<"${work}/said")
  [[ ${said} == *"$2"* && ${said} == *"$3"* ]] || {
    echo "bare-sheet.sh refused $4 without saying so: ${said}" >&2
    exit 1
  }
}
echo "a { color: red; }" >"${sheet}"
sheet_refused "${work}/broke" "out.css: esbuild refused it" "stand-in: refused" \
  "a sheet esbuild refuses"
sheet_refused "${work}/warns" "out.css: esbuild warned on it" "stand-in: careful" \
  "a sheet esbuild warns on"
sheet_refused "${work}/crashes" "out.css: esbuild exited 2 on it" "stand-in: crashed" \
  "a sheet esbuild crashes on"
# The real esbuild refuses a block left open one way or the other, and says
# why beneath; and keeps a comment marked to stay, which is refused.
echo "a { color: red" >"${sheet}"
sheet_refused "${esbuild}" "out.css: esbuild" "" "a sheet with a block left open"
[[ ${said} == *$'\n'?* ]] || {
  echo "bare-sheet.sh refused a block left open without what esbuild said: ${said}" >&2
  exit 1
}
echo "/*! kept */ a { color: red; }" >"${sheet}"
sheet_refused "${esbuild}" "out.css: holds /* once written again" "" \
  "a sheet keeping a comment"
# And writes a sheet's rules again without the rest of its comments.
printf '%s\n' '/* gone */' 'a { color: red; }' >"${sheet}"
bash "${bare_sheet}" "${esbuild}" "${sheet}" "${work}/bare/out.css"
if grep -q 'gone' "${work}/bare/out.css" || ! grep -q 'color: red' "${work}/bare/out.css"; then
  echo "bare-sheet.sh didn't write a sheet's rules without its comment" >&2
  exit 1
fi
# A site's build hands each of its sheets to bare-sheet.sh, the others
# passing, and stops on a refusal.
refused "${source}" "site.css: esbuild refused it" "a sheet esbuild refuses" "${work}/broke"

# kept-comments.sh fails on nothing to read and on a path that isn't a
# script, so a glob that missed can't pass.
if bash "${kept_comments}" 2>/dev/null; then
  echo "kept-comments.sh passed with no script to read" >&2
  exit 1
fi
if bash "${kept_comments}" "${work}/*.js" 2>/dev/null; then
  echo "kept-comments.sh passed a glob that matched nothing" >&2
  exit 1
fi
# A site's build names each compiled script still holding a comment marked
# to stay, here two, before it fails.
copied "${work}/script"
for each in site looks; do
  {
    echo '/*! kept */'
    cat "${source}/${each}.ts"
  } >"${work}/script/${each}.ts"
done
refused "${work}/script" "site.js: holds /*! once compiled" "a script keeping a comment"
[[ ${said} == *"looks.js: holds /*! once compiled"* ]] || {
  echo "the build named one script keeping a comment, not each: ${said}" >&2
  exit 1
}
# A script written by hand goes through the compiler, which drops its
# comments but keeps a string, a template and a pattern holding what reads
# like one, which a pattern couldn't tell apart.
copied "${work}/plain"
cat >"${work}/plain/extra.js" <<'EOF'
// gone
export const u = "http://x/*y*/";
export const t = `a // b`;
export const r = /\/\//u;
export const n = 1; // gone too
EOF
rm -rf "${work}/out"
bash "${build_with[@]}" "${work}/plain" "${looks[@]}"
grep -v gone "${work}/plain/extra.js" >"${work}/kept"
while IFS= read -r kept; do
  grep -qF "${kept}" "${work}/out/extra.js" || {
    echo "a script written by hand lost what only reads like a comment: ${kept}" >&2
    exit 1
  }
done <"${work}/kept"
grep -qF 'export const n = 1;' "${work}/out/extra.js" || {
  echo "a script written by hand lost the code before a comment ending its line" >&2
  exit 1
}
if grep -q 'gone' "${work}/out/extra.js"; then
  echo "a script written by hand reached the page with its comments" >&2
  exit 1
fi
