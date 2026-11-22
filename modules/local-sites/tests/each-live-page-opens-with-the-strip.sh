# shellcheck shell=bash
# Checks that localSite writes the strip every live page opens with: once,
# straight after the skip link, linking home one folder up and saying only
# that the page is an app, never a word that reads as how it stands. And
# that a page carrying a strip of its own fails its build with why, since
# two strips, or one saying more than the kind, is what this replaces. And
# that the page reads the theme's web kit from the one copy the sites share,
# with none of its own, and a page still reading its own kit/ fails too.
# usage: each-live-page-opens-with-the-strip.sh <built site> <site.sh> <tsc>
#   <node> <esbuild> <kit> <site source> <name>
set -euo pipefail

built=$1 build=$2 tsc=$3 node=$4 esbuild=$5 kit=$6 source=$7 name=$8
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
for sheet in "${built}"/*.css; do
  if grep -q '/\*' "${sheet}"; then
    echo "${sheet##*/} is sent with a comment" >&2
    exit 1
  fi
done

# What every build below is handed but the site.
build_with=("${build}" "${tsc}" "${node}" "${esbuild}" "${kit}")
looks=("${name}" "${work}/out" black white black white)

# A copy of the site's source, at `$1`, for a case to change.
copied() {
  cp -r "${source}" "$1"
  chmod -R u+w "$1"
}

# Builds the site at `$1` and expects it refused, saying `$2`; `$3` names the case.
refused() {
  rm -rf "${work}/out"
  if bash "${build_with[@]}" "$1" "${looks[@]}" 2>"${work}/said"; then
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

# A sheet esbuild warns on, or can't read, is refused, saying which, and a
# comment it keeps because it's marked to stay is refused too.
copied "${work}/unclosed"
echo "a { color: red" >>"${work}/unclosed/site.css"
refused "${work}/unclosed" "site.css: esbuild wouldn't write it again cleanly" \
  "a sheet esbuild warns on"
copied "${work}/marked"
echo "/*! kept */ a { color: red; }" >>"${work}/marked/site.css"
refused "${work}/marked" "site.css: a comment is still in it" "a sheet keeping a comment"
