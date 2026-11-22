# shellcheck shell=bash
# Checks that localSite writes the strip every live page opens with: once,
# straight after the skip link, linking home one folder up and saying only
# that the page is an app, never a word that reads as how it stands. And
# that a page carrying a strip of its own fails its build with why, since
# two strips, or one saying more than the kind, is what this replaces. And
# that the page reads the theme's web kit from the one copy the sites share,
# with none of its own, and a page still reading its own kit/ fails too.
# And that the page's sheets and scripts reach it without their comments,
# and a sheet esbuild can't read or warns on, or a sheet or a script keeping
# a comment, fails, saying which.
# usage: each-live-page-opens-with-the-strip.sh <built site> <site.sh> <tsc>
#   <node> <esbuild> <bare-sheet.sh> <kept-comments.sh> <kit> <site source>
#   <name>
set -euo pipefail

built=$1 build=$2 tsc=$3 node=$4 esbuild=$5 bare_sheet=$6 kept_comments=$7 kit=$8
source=$9 name=${10}
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

# The built sheets hold no comment, so each sheet's write in place landed.
commented=$(grep -l '/\*' "${built}"/*.css || true)
[[ -z ${commented} ]] || {
  echo "a built sheet still holds a comment: ${commented}" >&2
  exit 1
}

# bare-sheet.sh on one sheet: a sheet esbuild can't read is refused, saying
# which and what esbuild said, and so is one it only warns on, and one it
# exits on otherwise. Each is told by an esbuild standing in, since which of
# these a given slip is belongs to esbuild's release; each fails only on
# site.css and writes any other as it is, so a site's other sheets pass.
# Each names this bash, since a build's sandbox has no /usr/bin/env.
stand_in() {
  echo "#!${BASH}"
  cat <<'EOF'
if [[ ${*: -1} == */site.css ]]; then
EOF
  echo "$1"
  cat <<'EOF'
fi
cat "${@: -1}"
EOF
}
stand_in '  echo "stand-in: unreadable" >&2; exit 1' >"${work}/broke"
stand_in '  echo "stand-in: careful" >&2' >"${work}/warns"
stand_in '  echo "stand-in: crashed" >&2; exit 3' >"${work}/crashes"
chmod +x "${work}/broke" "${work}/warns" "${work}/crashes"
mkdir "${work}/sheets" "${work}/bare"
sheet=${work}/sheets/site.css

# Runs bare-sheet.sh on `sheet` with the esbuild `$1` and expects it refused,
# saying `$2` and what esbuild said, `$3`; `$4` names the case.
sheet_refused() {
  if bash "${bare_sheet}" "$1" "${sheet}" "${work}/bare/site.css" 2>"${work}/said"; then
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
sheet_refused "${work}/broke" "site.css: esbuild couldn't read it" "stand-in: unreadable" \
  "a sheet esbuild can't read"
sheet_refused "${work}/warns" "site.css: esbuild warned on it" "stand-in: careful" \
  "a sheet esbuild warns on"
sheet_refused "${work}/crashes" "site.css: esbuild exited 3 on it" "stand-in: crashed" \
  "a sheet esbuild exits on"
# The real esbuild refuses a block left open one way or the other, and
# keeps a comment marked to stay, which is refused.
echo "a { color: red" >"${sheet}"
sheet_refused "${esbuild}" "site.css: esbuild" "" "a sheet with a block left open"
echo "/*! kept */ a { color: red; }" >"${sheet}"
sheet_refused "${esbuild}" "site.css: holds /* once written again" "" \
  "a sheet keeping a comment"
# And writes a sheet's rules again without the rest of its comments.
printf '%s\n' '/* gone */' 'a { color: red; }' >"${sheet}"
bash "${bare_sheet}" "${esbuild}" "${sheet}" "${work}/bare/site.css"
if grep -q 'gone' "${work}/bare/site.css" || ! grep -q 'color: red' "${work}/bare/site.css"; then
  echo "bare-sheet.sh didn't write a sheet's rules without its comment" >&2
  exit 1
fi
# A site's build hands its sheets to bare-sheet.sh, and stops on its refusal.
refused "${source}" "site.css: esbuild couldn't read it" "a sheet esbuild can't read" \
  "${work}/broke"

# kept-comments.sh names each script holding a comment marked to stay, and
# passes one holding none.
printf '%s\n' '/*! kept */' 'export const kept = 1;' >"${work}/kept.js"
echo 'export const plain = 1;' >"${work}/plain.js"
if bash "${kept_comments}" "${work}/plain.js" "${work}/kept.js" 2>"${work}/said"; then
  echo "a script keeping a comment passed kept-comments.sh" >&2
  exit 1
fi
said=$(<"${work}/said")
[[ ${said} == "kept.js: holds /*! once compiled"* ]] || {
  echo "kept-comments.sh refused a script without naming it alone: ${said}" >&2
  exit 1
}
bash "${kept_comments}" "${work}/plain.js"
# A site's build hands it the scripts it compiled from TypeScript, and those
# written by hand, which the compiler writes again without the rest of their
# comments.
copied "${work}/script"
{
  echo '/*! kept */'
  cat "${source}/site.ts"
} >"${work}/script/site.ts"
refused "${work}/script" "site.js: holds /*! once compiled" "a script keeping a comment"
copied "${work}/written"
printf '%s\n' '/*! kept */' 'export const kept = 1;' >"${work}/written/extra.js"
refused "${work}/written" "extra.js: holds /*! once compiled" \
  "a script written by hand keeping a comment"
copied "${work}/plain"
printf '%s\n' '// gone' 'export const plain = 1;' >"${work}/plain/extra.js"
rm -rf "${work}/out"
bash "${build_with[@]}" "${work}/plain" "${looks[@]}"
if grep -q 'gone' "${work}/out/extra.js" || ! grep -q 'plain = 1' "${work}/out/extra.js"; then
  echo "a script written by hand reached the page with its comment" >&2
  exit 1
fi
