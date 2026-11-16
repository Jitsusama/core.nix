# shellcheck shell=bash
# Checks that localSite writes the strip every live page opens with: once,
# straight after the skip link, linking home one folder up and saying only
# that the page is an app, never a word that reads as how it stands. And
# that a page carrying a strip of its own fails its build with why, since
# two strips, or one saying more than the kind, is what this replaces.
# usage: each-live-page-opens-with-the-strip.sh <built site> <site.sh> <tsc>
#   <node> <kit> <site source> <name> <web kit>
set -euo pipefail

built=$1 build=$2 tsc=$3 node=$4 kit=$5 source=$6 name=$7 web=$8
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

# A page that brings its own strip is refused, saying which page and why.
cp -r "${source}" "${work}/site"
chmod -R u+w "${work}/site"
sed -i '/<a class="skip"/a <nav class="sites-strip"><span>App</span> <span>live</span></nav>' \
  "${work}/site/index.html"
if bash "${build}" "${tsc}" "${node}" "${kit}" "${work}/site" "${name}" "${web}" "${work}/out" \
  black white black white 2>"${work}/said"; then
  echo "a page with a strip of its own built anyway" >&2
  exit 1
fi
refused=$(<"${work}/said")
[[ ${refused} == *'index.html carries its own strip'* ]] || {
  echo "the build refused a page's own strip without saying so: ${refused}" >&2
  exit 1
}
