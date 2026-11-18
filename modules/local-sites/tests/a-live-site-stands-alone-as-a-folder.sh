# shellcheck shell=bash
# A live site laid out as a folder that stands alone has its page at the
# root, whole, reading the theme and the kit from its own web/ and kit/,
# with no strip. A site that still loads something from outside the folder
# fails its build, saying which file and which address, and leaves no folder
# behind: a page's link, image, object, poster, background or xlink:href, in
# any case, quoted or not and past a quoted >; a style's url() or @import in
# any case, inline or in a stylesheet; a module's import, bare or dynamic,
# or its fetch, the global one's too, spaced from its parenthesis, holding
# the other quote, in its file or inline, after or between comments, wrapped
# over lines or built from the root; an address at a root, on another host,
# of data: or built whole as it runs; a file the folder lacks; a srcset, an
# import map or an image-set(). So does a strip whose ends don't pair, share
# a line or no longer read, a site with no index.html, and a folder laid over
# one already there, which stays. A link to another of the sites is followed
# rather than loaded, so it stays, as do a data-src, an address with a query
# or naming a folder, one a module builds from a start as it runs in its file
# or inline, a page's new URL(), another object's fetch method and words
# that only read like an import.
# usage: a-live-site-stands-alone-as-a-folder.sh <folder> <standalone.sh> <site>
#   <kit> <web kit>
set -euo pipefail

folder=$1 standalone=$2 site=$3 kit=$4 web=$5

fail() {
  echo "a-live-site-stands-alone-as-a-folder: $*" >&2
  exit 1
}

[[ -f ${folder}/index.html ]] || fail "the folder has no index.html at its root"
grep -qF 'href="./web/theme.css"' "${folder}/index.html" ||
  fail "the page doesn't read the theme from the folder's own web/"
grep -qF 'href="./kit/switch.css"' "${folder}/index.html" ||
  fail "the page doesn't read the switch from the folder's own kit/"
[[ -f ${folder}/web/fonts/fraunces.woff2 ]] || fail "the folder lacks the web kit's fonts"
grep -qF '</html>' "${folder}/index.html" || fail "the page lost its end with the strip"
grep -qF '<main' "${folder}/index.html" || fail "the page lost its main part with the strip"
if grep -qF 'sites-strip' "${folder}/index.html"; then
  fail "the page still opens with the strip, whose home isn't in the folder"
fi

work=$(mktemp -d)

# A copy of the site with one file changed by a sed script, laid out.
changed() {
  local name=$1 file=$2 script=$3
  cp -r "${site}" "${work}/${name}"
  chmod -R u+w "${work}/${name}"
  sed -i "${script}" "${work}/${name}/${file}"
}

# Fails unless the site, with one file changed, fails to lay out and says
# what it was told to.
refuses() {
  local name=$1 file=$2 script=$3 says=$4
  changed "${name}" "${file}" "${script}"
  if bash "${standalone}" "${work}/${name}" "${kit}" "${web}" "${work}/${name}-out" \
    2>"${work}/${name}-said"; then
    fail "${name}: the site built anyway"
  fi
  local said
  said=$(<"${work}/${name}-said")
  [[ ${said} == *"${says}"* ]] || fail "${name}: the build didn't say \"${says}\", but: ${said}"
  [[ ! -e ${work}/${name}-out ]] || fail "${name}: the refused folder was left behind"
}

refuses climbing index.html \
  's|</head>|<link rel="stylesheet" href="../elsewhere.css"></head>|' \
  "index.html: ../elsewhere.css, outside the folder"
refuses rooted index.html \
  's|</head>|<link rel="stylesheet" href="/local-sites/web/theme.css"></head>|' \
  "index.html: /local-sites/web/theme.css, outside the folder"
refuses off-site index.html \
  's|</head>|<link rel="stylesheet" href="https://elsewhere.example/a.css"></head>|' \
  "index.html: https://elsewhere.example/a.css, outside the folder"
refuses lacking index.html \
  's|</head>|<link rel="stylesheet" href="./missing.css"></head>|' \
  "index.html: ./missing.css, which the folder lacks"
refuses inline-style index.html \
  "s|</body>|<div style=\"background: url('../ground.png')\"></div></body>|" \
  "index.html: ../ground.png, outside the folder"
refuses srcset index.html \
  's|</body>|<img alt="" srcset="./favicon.svg 1x"></body>|' \
  "index.html: a srcset, an import map"
refuses import-map index.html \
  's|</head>|<script type="importmap">{}</script></head>|' \
  "index.html: a srcset, an import map"
refuses upper-case index.html \
  's|</head>|<LINK REL="stylesheet" HREF = "../up.css"></head>|' \
  "index.html: ../up.css, outside the folder"
refuses unquoted index.html 's|</head>|<link rel=stylesheet href=../bare.css></head>|' \
  "index.html: ../bare.css, outside the folder"
refuses inline-import index.html 's|</head>|<style>@import "../x.css";</style></head>|' \
  "index.html: ../x.css, outside the folder"
refuses inline-module index.html \
  's|</body>|<script type="module">import "../y.js";</script></body>|' \
  "index.html: ../y.js, outside the folder"
refuses unclosed index.html '\|<!-- /sites strip -->|d' \
  "index.html: the strip's ends don't pair"
opens='<!-- sites strip: localSite writes this -->' closes='<!-- /sites strip -->'
refuses swapped index.html "s|${closes}|@@|; s|${opens}|${closes}|; s|@@|${opens}|" \
  "index.html: the strip's ends don't pair"
refuses renamed index.html \
  's|sites strip: localSite writes this|strip by localSite|; s|/sites strip|/strip by localSite|' \
  "index.html: the strip is still here"
refuses one-line index.html "s|${opens}|${opens}${closes}|" \
  "index.html: the strip's ends don't pair"
refuses past-a-quote index.html \
  's|</head>|<link title="a > b" rel="stylesheet" href="../past.css"></head>|' \
  "index.html: ../past.css, outside the folder"
refuses poster index.html 's|</body>|<video poster="../still.png"></video></body>|' \
  "index.html: ../still.png, outside the folder"
refuses image-set site.css "\$a body { background: image-set(\"./a.png\" 1x); }" \
  "site.css: a srcset, an import map or an image-set()"
refuses upper-url site.css "\$a body { background: URL(../shout.png); }" \
  "site.css: ../shout.png, outside the folder"
refuses tight-import site.css "\$a @IMPORT\"../tight.css\";" \
  "site.css: ../tight.css, outside the folder"
refuses global-fetch site.js "\$a void globalThis.fetch(\"../global.json\");" \
  "site.js: ../global.json, outside the folder"
refuses image index.html 's|</body>|<img alt="" src="../up.png"></body>|' \
  "index.html: ../up.png, outside the folder"
refuses object index.html 's|</body>|<object data="../drawn.svg"></object></body>|' \
  "index.html: ../drawn.svg, outside the folder"
refuses xlink index.html 's|</body>|<svg><use xlink:href="../marks.svg#a"></use></svg></body>|' \
  "index.html: ../marks.svg#a, outside the folder"
refuses background index.html 's|</body>|<table background="../ground.png"></table></body>|' \
  "index.html: ../ground.png, outside the folder"
refuses inline-fetch index.html \
  's|</body>|<script type="module">void fetch("../inline.json");</script></body>|' \
  "index.html: ../inline.json, outside the folder"
refuses mixed-quotes site.js "\$a void fetch(\"../it's.json\");" \
  "site.js: ../it's.json, outside the folder"
refuses spaced-fetch site.js "\$a void fetch (\"../spaced.json\");" \
  "site.js: ../spaced.json, outside the folder"
refuses wrapped site.js "\$a void fetch(\n  \"../wrapped.json\");" \
  "site.js: ../wrapped.json, outside the folder"
refuses built-whole site.js "\$a void fetch(\`\${where}\`);" \
  "site.js: \${where}, built whole as it runs"
refuses after-comment site.js "\$a /* why */ import \"../after.js\";" \
  "site.js: ../after.js, outside the folder"
refuses between-comments site.js "\$a /* a */ import \"../between.js\"; /* b */" \
  "site.js: ../between.js, outside the folder"
refuses rooted-template site.js "\$a void fetch(\`/feed/\${name}\`);" \
  "site.js: /feed/\${name}, outside the folder"
refuses data site.js "\$a void import(\"data:text/javascript,1\");" \
  "site.js: a data: address"
refuses styled-import site.css "\$a @import \"../x.css\";" \
  "site.css: ../x.css, outside the folder"
refuses importing site.js 's|from "./draw.js"|from "../draw.js"|' \
  "site.js: ../draw.js, outside the folder"
refuses bare-import site.js "\$a import \"../side.js\";" \
  "site.js: ../side.js, outside the folder"
refuses dynamic-import site.js "\$a void import(\"../later.js\");" \
  "site.js: ../later.js, outside the folder"
refuses fetching site.js "\$a void fetch(\"../sites.json\");" \
  "site.js: ../sites.json, outside the folder"
refuses styled site.css "\$a body { background: url(../ground.png); }" \
  "site.css: ../ground.png, outside the folder"

# A folder laid over one already there would nest inside it.
if bash "${standalone}" "${site}" "${kit}" "${web}" "${work}" 2>"${work}/over-said"; then
  fail "a site laid over a folder already there built anyway"
fi
grep -qF 'is already there' "${work}/over-said" ||
  fail "a site laid over a folder already there didn't say so: $(<"${work}/over-said")"
[[ -f ${work}/over-said ]] || fail "a refusal to lay a site over a folder took the folder"

# A site with no page to stand at the root fails, and leaves no folder.
cp -r "${site}" "${work}/no-index"
chmod -R u+w "${work}/no-index"
rm "${work}/no-index/index.html"
if bash "${standalone}" "${work}/no-index" "${kit}" "${web}" "${work}/no-index-out" \
  2>"${work}/no-index-said"; then
  fail "a site with no index.html built anyway"
fi
grep -qF 'no index.html' "${work}/no-index-said" ||
  fail "a site with no index.html didn't say so: $(<"${work}/no-index-said")"
[[ ! -e ${work}/no-index-out ]] || fail "a site with no index.html left its folder behind"

# A link to another site, and what only reads like a load, stay.
linked='<a href="../code/">Code</a><link rel="alternate" href="./">'
linked+='<link rel="stylesheet" href="./site.css?v=1"><p data-src="../said.png">said</p>'
linked+='<script type="module">const at = new URL("../up/", location.href);'
linked+="void fetch(\`./\${at.hash}.json\`);</script>"
changed linking index.html "s|</body>|${linked}</body>|"
printf '%s\n' '/** Read from "../nowhere.js" once. */' 'export const words = ["from", "in"];' \
  "export const rows = (name) => fetch(\`./\${name}.json\`);" \
  'export const asked = (them) => them.fetch("../theirs.json");' \
  'export const other = (notwindow) => notwindow.fetch("../theirs.json");' \
  >>"${work}/linking/site.js"
printf '%s\n' '/* <!-- sites strip: localSite writes this --> */' '.kept {}' \
  >>"${work}/linking/site.css"
bash "${standalone}" "${work}/linking" "${kit}" "${web}" "${work}/linking-out" ||
  fail "a site linking to another site, or with what only reads like a load, didn't build"
grep -qF '.kept {}' "${work}/linking-out/site.css" ||
  fail "a stylesheet naming the strip's marker lost the rules after it"
grep -qF '<a href="../code/">Code</a>' "${work}/linking-out/index.html" ||
  fail "the link to another site didn't stay as it was"
echo "the site stands alone as a folder"
