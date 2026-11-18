# shellcheck shell=bash
# Lays a built live site out as one folder that stands alone (site.nix says
# why): its page at the root, the kit and the theme's web kit copied in
# beside it as kit/ and web/, everything the page loads within the folder,
# and the strip left out, since the home it leads to isn't in the folder.
#
# The folder's pages, styles and modules, the kit's and web kit's whole
# copies among them, are read for the addresses written into them, and each
# is judged: a tag's src, href, xlink:href, poster, data and background, in
# any case, quoted or not (but a link's href, which is followed rather than
# loaded), a style's url() and @import, and a module's imports, static,
# bare and dynamic, and its fetches, in its own file or inline in a page,
# its comments left out. One that starts at a root, names another host,
# climbs out of the folder, is a data: address, which may load from
# anywhere, is built whole as a module runs, or names a file the folder
# lacks fails the build, each named with the file it's in, so a site that
# grows a new way out says so. An address a module builds from a start, by
# a template or by adding to a string, is judged only by whether that start
# climbs out; a tag's or a style's is taken as written. A srcset, an import
# map or an image-set(), which this doesn't follow, fails it too, and so
# does a page's prose that only reads like a load, or a tag's link to
# another site that isn't a link's, which is worded otherwise rather than
# guessed at.
#
# What this doesn't read it can't judge: the .svg files the folder serves,
# a link's own ping, an address handed to anything but import and fetch (a
# socket, an event source, a request, a worker, new URL, markup a module
# draws), and one handed to fetch by a name, as Code reads its rows. A site
# that loads one so fails on another host as any lost read does; the feed's
# socket is one, asked of the root on purpose. A link a module draws to
# another of the sites, or a page's link to one, stays as it is: it's where
# the reader may go next, not a file the page loads, and only the sites'
# server holds the others.
# usage: standalone.sh <site> <kit> <web kit> <out>
set -euo pipefail

site=$1 kit=$2 web=$3 out=$4

# Laid over an earlier folder, the copy would keep files the site no longer
# has, so the folder is made here, in one step, and two builds can't share
# one.
if [[ -e ${out} ]] || ! mkdir -- "${out}"; then
  echo "${out} is already there; a site is laid out into a fresh folder" >&2
  exit 1
fi
# A refused folder is half rewritten, so it goes with the refusal, and
# the lists the reading keeps go however it ends.
listing='' addresses=''
tidy() {
  local status=$?
  rm -f "${listing}" "${addresses}"
  if ((status != 0)); then
    rm -rf "${out}"
  fi
}
trap tidy EXIT

# Copied with every link followed, since a host that serves a folder may
# follow none, and writable, since the addresses are rewritten in place.
cp -rL "${site}/." "${out}"
chmod -R u+w "${out}"
cp -rL "${kit}" "${out}/kit"
cp -rL "${web}" "${out}/web"
chmod -R u+w "${out}/kit" "${out}/web"

# The markers site.sh writes round the strip, each on its own line. Deleting
# from an opening that never closes on a later line would take the rest of
# the page, so each must, before the next opens.
strip_opens='<!-- sites strip: localSite writes this -->'
strip_closes='<!-- /sites strip -->'
strips=()
unpaired="the strip's ends don't pair, each on its own line and closing before the next opens"
for file in "${out}"/*.html; do
  [[ -f ${file} ]] || continue
  paired=$(awk -v opens="${strip_opens}" -v closes="${strip_closes}" '
    index($0, opens) && index($0, closes) { bad = 1 }
    index($0, opens) { if (open) bad = 1; open = 1; next }
    index($0, closes) { if (!open) bad = 1; open = 0 }
    END { print (bad || open) ? "no" : "yes" }' "${file}")
  [[ ${paired} == yes ]] ||
    strips+=("${file#"${out}/"}: ${unpaired}")
done

# A page names the shared copy one folder up, and a compiled module two
# (site.nix says why); both land on the folder's own copy now.
# Only a page has a strip, and only its markers were paired.
if ((${#strips[@]} == 0)); then
  for file in "${out}"/*.html "${out}"/*.css "${out}"/*.js; do
    [[ -f ${file} ]] || continue
    sed -i -e 's|\(\.\./\)\{1,2\}local-sites/kit/|./kit/|g' \
      -e 's|\(\.\./\)\{1,2\}local-sites/web/|./web/|g' "${file}"
    if [[ ${file} == *.html ]]; then
      sed -i -e "\\|${strip_opens}|,\\|${strip_closes}|d" "${file}"
    fi
  done
  still="the strip is still here, so its markers no longer read as site.sh writes them"
  for file in "${out}"/*.html; do
    [[ -f ${file} ]] || continue
    if grep -qF 'sites-strip' "${file}"; then
      strips+=("${file#"${out}/"}: ${still}")
    fi
  done
fi
if ((${#strips[@]} > 0)); then
  printf '%s\n' "these pages' strips can't be taken out whole:" "${strips[@]}" >&2
  exit 1
fi

quote="[\"'\`]"
# A string, closed by the quote that opened it.
quoted="(\"[^\"]*\"|'[^']*'|\`[^\`]*\`)"
# An attribute's value, quoted any way or not at all.
value="(${quoted}|[^\"'\`[:space:]>]+)"
# The attributes a tag loads from.
loading="(src|href|xlink:href|poster|data|background)"
# A tag, running past a > inside a quoted value.
tag="<[a-zA-Z]([^>\"']|\"[^\"]*\"|'[^']*')*>"
# The addresses a style loads, in the text $1, each after `style`. A
# module's new URL() in a page reads the same in any case, and isn't a
# style's.
styled() {
  grep -oiE "(new[[:space:]]+)?url\\([^)]*\\)" <<<"$1" | grep -viE '^new' |
    sed -E "s/^[uU][rR][lL]\\([[:space:]]*${quote}?/style /; s/${quote}?[[:space:]]*\\)\$//" ||
    true
  grep -oiE "@import[[:space:]]*${quoted}" <<<"$1" |
    sed -E "s/^@[a-zA-Z]+[[:space:]]*./style /; s/.\$//" || true
}
# The addresses a module loads, in the text $1, each after `module`, or
# after `joined` where a string is added to: a static import starts a
# statement or follows what it names, and takes only plain quotes, where a
# dynamic one, or a fetch that isn't some other object's method, takes any.
scripted() {
  grep -oE "(^|[[:space:];}>])(from|import)[[:space:]]*[\"'][^\"']*[\"']" <<<"$1" |
    sed -E "s/^[[:space:];}>]?(from|import)[[:space:]]*./module /; s/.\$//" || true
  local called="(^|[^.[:alnum:]_\$])((window|globalThis|self)\\.)?(import|fetch)[[:space:]]*\\("
  local joined="/\\+\$/{s/^(.*)[^[:space:]][[:space:]]*\\+\$/joined \\1/;b}"
  grep -oE "${called}[[:space:]]*${quoted}([[:space:]]*\\+)?" <<<"$1" |
    sed -E "s/^.*(import|fetch)[[:space:]]*\\([[:space:]]*.//; ${joined}; s/^(.*).\$/module \\1/" ||
    true
}
# The addresses a file loads, one a line, each after how it loads it.
loads() {
  local file=$1 text
  case ${file} in
  *.html)
    # Read whole, since a tag may run over lines; a link's href is
    # followed, not loaded, and only that leaves its tag, so a quote a
    # link leaves open hides no tag after it. Its inline styles and
    # scripts are read too.
    text=$(tr '\n' ' ' <"${file}")
    grep -oE "${tag}" <<<"${text}" |
      sed -E "/^<[aA][[:space:]>]/s/[[:space:]]href[[:space:]]*=[[:space:]]*${value}//gI" |
      grep -oiE "[[:space:]]${loading}[[:space:]]*=[[:space:]]*${value}" |
      sed -E "s/^[[:space:]][a-zA-Z:]+[[:space:]]*=[[:space:]]*/tag /; s/^tag ${quote}/tag /" |
      sed -E "s/${quote}\$//" || true
    styled "${text}"
    scripted "${text}"
    ;;
  *.css)
    text=$(tr '\n' ' ' <"${file}")
    styled "${text}"
    ;;
  *.js)
    # A comment's words may read like an import, so every comment is left
    # out: a line's after the line is read, a block's once the module is
    # read whole, since an address may wrap onto the next line.
    text=$(grep -vE '^[[:space:]]*//' "${file}" | tr '\n' ' ' |
      sed -E 's|/\*[^*]*\*+([^/*][^*]*\*+)*/| |g' || true)
    scripted "${text}"
    ;;
  *) ;;
  esac
}

problems=()
unfollowed="a srcset, an import map or an image-set(), which a folder standing alone doesn't follow"
whole="built whole as it runs, so it can't be judged"
listing=$(mktemp)
addresses=$(mktemp)
find "${out}" -type f \( -name '*.html' -o -name '*.css' -o -name '*.js' \) | sort >"${listing}"
while IFS= read -r file; do
  named=${file#"${out}/"}
  loads "${file}" >"${addresses}"
  if grep -qiE "srcset[[:space:]]*=|type[[:space:]]*=[[:space:]]*${quote}?importmap|image-set\\(" \
    "${file}"; then
    problems+=("${named}: ${unfollowed}")
  fi
  while IFS=' ' read -r how address; do
    if [[ ${how} == joined && -z ${address} ]]; then
      problems+=("${named}: a string added to, ${whole}")
      continue
    fi
    case ${address} in
    '' | '#'*) continue ;;
    [dD][aA][tT][aA]:*)
      problems+=("${named}: a data: address, which may load from anywhere")
      continue
      ;;
    /* | *://*)
      problems+=("${named}: ${address}, outside the folder")
      continue
      ;;
    *) ;;
    esac
    # A module's address built as it runs, in its file or inline in a page,
    # is judged only by whether where it starts climbs out, and one built
    # whole can't be judged at all.
    built=no
    [[ ${how} != joined ]] || built=yes
    if [[ ${how} == module && ${address} == *"\${"* ]]; then
      if [[ ${address} == "\${"* ]]; then
        problems+=("${named}: ${address}, ${whole}")
        continue
      fi
      built=yes
    fi
    target=${address%%[?#]*}
    case $(realpath -m --relative-to="${out}" "${file%/*}/${target}") in
    .. | ../*)
      problems+=("${named}: ${address}, outside the folder")
      continue
      ;;
    *) ;;
    esac
    [[ ${built} == no ]] || continue
    [[ -z ${target} || ${target} == */ ]] && target="${target}index.html"
    [[ -e ${file%/*}/${target} ]] || problems+=("${named}: ${address}, which the folder lacks")
  done <"${addresses}"
done <"${listing}"

if ((${#problems[@]} > 0)); then
  printf '%s\n' "these still load what the folder doesn't hold:" "${problems[@]}" >&2
  exit 1
fi
if [[ ! -f ${out}/index.html ]]; then
  echo "the site has no index.html to stand at the folder's root" >&2
  exit 1
fi
