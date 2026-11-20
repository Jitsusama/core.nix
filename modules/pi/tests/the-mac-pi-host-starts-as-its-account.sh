# shellcheck shell=bash
# Runs a Mac's pi host start (modules/pi/darwin.nix) on Linux, under each
# POSIX shell given, with the Mac's own programs stood in for: getconf, id
# and sleep by stand-ins the test steers or that say they ran, the rest by
# Linux's, and the host's node by one that writes down its pid, its words
# and its environment. The start has to pass shellcheck as sh, move only a
# log past 10 MB aside, wait before a fourth start in ten minutes, say why
# and fail when the harness is missing or the temporary folder isn't the
# account's alone, and otherwise become the node in its own place, with the
# words it was given and every value of the realm's environment whole,
# whatever the shell would read in it, and none of the variables that would
# run code as node starts.
# usage: the-mac-pi-host-starts-as-its-account.sh <start> <entry> <shell>...
set -euo pipefail

start=$1
entry=$2
shift 2

fail() {
  echo "the Mac's pi host start $*" >&2
  exit 1
}

# Every lint but the note on a dollar in single quotes, since quoting a
# value that holds one is the point.
shellcheck --shell=sh --exclude=SC2016 "${start}" || fail "doesn't pass shellcheck as sh"

work=$(mktemp -d)
stand=${work}/stand-ins
mkdir -p "${stand}"
env_program=$(command -v env)
stat_program=$(command -v stat)
id_program=$(command -v id)
mv_program=$(command -v mv)
mkdir_program=$(command -v mkdir)
date_program=$(command -v date)
sed_program=$(command -v sed)
tail_program=$(command -v tail)

# Prints the folder the test gives it, as getconf prints the account's own,
# or fails as getconf does when it has none.
cat >"${stand}/getconf" <<EOF
#!${BASH}
[ "\$1" = DARWIN_USER_TEMP_DIR ] || exit 2
[ -n "\${GIVEN_TEMPORARY:-}" ] || exit 1
printf '%s\n' "\${GIVEN_TEMPORARY}"
EOF

# The account's uid, or the one the test gives it.
cat >"${stand}/id" <<EOF
#!${BASH}
[ "\$1" = -u ] || exit 2
printf '%s\n' "\${GIVEN_UID:-\$(${id_program} -u)}"
EOF

# Says it slept rather than sleeping.
cat >"${stand}/sleep" <<EOF
#!${BASH}
printf '%s\n' "\$*" >>"${work}/slept"
EOF

# The host's node: writes down its pid, its words and its environment, and
# prints a line, which has to reach the log.
cat >"${stand}/node" <<EOF
#!${BASH}
printf '%s\n' "\$\$" >"${work}/pid"
printf '%s\n' "\$@" >"${work}/words"
${env_program} -0 >"${work}/environment"
echo "node ran"
EOF
chmod +x "${stand}"/*

logs=${work}/Logs
log=${logs}/host.log
present=${work}/entry
touch "${present}"

# The start as the Mac reads it, with each of its Mac paths led elsewhere.
sed \
  -e "s|/usr/bin/getconf|${stand}/getconf|g" \
  -e "s|/usr/bin/stat -f %z|${stat_program} -c %s|g" \
  -e "s|/usr/bin/stat -f '%u %Lp'|${stat_program} -c '%u %a'|g" \
  -e "s|/usr/bin/id|${stand}/id|g" \
  -e "s|/bin/sleep|${stand}/sleep|g" \
  -e "s|/bin/mv|${mv_program}|g" \
  -e "s|/bin/mkdir|${mkdir_program}|g" \
  -e "s|/bin/date|${date_program}|g" \
  -e "s|/usr/bin/sed|${sed_program}|g" \
  -e "s|/usr/bin/tail|${tail_program}|g" \
  -e "s|/usr/bin/env|${env_program}|g" \
  -e "s|/Users/someone/Library/Logs/pi-host-one-attended.log|${log}|g" \
  -e "s|${entry}|${present}|g" \
  -e "s|/nix/store/[a-z0-9]*-pi-host/bin/pi-host|${stand}/node|g" \
  "${start}" >"${work}/start"

grep -q "${stand}/node" "${work}/start" || fail "doesn't become the realm's node named pi-host"

# The words the host's node has to be given.
expected_words=$(printf '%s\n' \
  --disable-sigusr1 \
  --disable-warning=MODULE_TYPELESS_PACKAGE_JSON \
  --max-old-space-size=8192 \
  "${present}" \
  attended)

# Fails unless the environment the node was given holds the name with the
# value, saying what it held instead.
expect() {
  local value
  value=$(tr '\0' '\n' <"${work}/environment" | sed -n "s|^$1=||p")
  [[ ${value} == "$2" ]] || fail "under ${name} gave $1 '${value}', not '$2'"
}

# Fails unless the environment the node was given holds the name with a
# value matching the pattern.
expect_like() {
  local value
  value=$(tr '\0' '\n' <"${work}/environment" | sed -n "s|^$1=||p")
  [[ ${value} =~ $2 ]] || fail "under ${name} gave $1 '${value}'"
}

# How long the start slept, or nothing when it didn't.
slept() {
  if [[ -e "${work}/slept" ]]; then
    cat "${work}/slept"
  fi
}

# Clears the log, the old one and the record of starts, and the logs'
# folder itself, which the start makes.
fresh() {
  rm -rf "${logs}"
}

# Runs the start under the shell with the variables given and no others, as
# launchd gives a daemon next to none, so nothing the test runs under can
# pass for what the start gives, after clearing what node and sleep wrote
# last time.
run() {
  rm -f "${work}/pid" "${work}/words" "${work}/environment" "${work}/slept"
  "${env_program}" -i "$@" "${shell}" "${work}/start"
}

# Fails unless the start, given a temporary folder, refuses it, says so in
# a log of its own and never runs node.
refused() {
  local what=$1
  shift
  fresh
  if run "$@"; then
    fail "under ${name} started in ${what}"
  fi
  grep -q "temporary folder wasn't found, or isn't its alone" "${log}" ||
    fail "under ${name} didn't say why it refused ${what}"
  [[ ! -e "${work}/words" ]] || fail "under ${name} ran node in ${what}"
}

for shell in "$@"; do
  name=$(basename "${shell}")
  temporary=${work}/temporary
  mkdir -p "${temporary}"
  chmod 700 "${temporary}"
  # A Mac's getconf ends the folder with a slash.
  given=${temporary}/

  # No harness yet: it says so, sleeps ten minutes and fails, so launchd
  # looks again.
  fresh
  mv "${present}" "${present}.away"
  if run GIVEN_TEMPORARY="${given}"; then
    fail "under ${name} started with no harness"
  fi
  mv "${present}.away" "${present}"
  grep -q "the harness has no host here yet" "${log}" ||
    fail "under ${name} didn't say the harness is missing"
  slept=$(slept)
  [[ ${slept} == 600 ]] || fail "under ${name} slept '${slept}', not ten minutes, for the harness"
  [[ ! -e "${work}/words" ]] || fail "under ${name} ran node with no harness"

  # A temporary folder that isn't the account's alone is refused, rather
  # than putting the host's socket where another may reach it.
  refused "no temporary folder"
  refused "a temporary folder that isn't there" GIVEN_TEMPORARY="${work}/not-there/"
  ln -sfn "${temporary}" "${work}/link"
  refused "a temporary folder that is a link" GIVEN_TEMPORARY="${work}/link/"
  mkdir -p "${work}/open"
  chmod 755 "${work}/open"
  refused "a temporary folder others may read" GIVEN_TEMPORARY="${work}/open/"
  refused "another account's temporary folder" GIVEN_TEMPORARY="${given}" GIVEN_UID=99999

  # Everything in place: the start becomes node in its own place, so the
  # pid launchd signals is node's, and a short log stays where it is.
  fresh
  mkdir -p "${logs}"
  echo "an earlier start" >"${log}"
  rm -f "${work}/pid" "${work}/words" "${work}/environment" "${work}/slept"
  # Each variable that would run code as node starts is given, as
  # launchd's or a machine's own could be, and none may reach node. The
  # loader's are empty, so Linux's loader, which reads them here, loads
  # nothing and says nothing; set, they still have to be cleared.
  "${env_program}" -i NODE_OPTIONS="--require /x.js" NODE_PATH=/x NODE_COMPILE_CACHE=/x \
    LD_PRELOAD= LD_AUDIT= LD_LIBRARY_PATH= OPENSSL_CONF=/x.cnf OPENSSL_MODULES=/x \
    GIVEN_TEMPORARY="${given}" "${shell}" "${work}/start" &
  started=$!
  wait "${started}" || fail "under ${name} failed with everything in place"
  pid=$(<"${work}/pid")
  [[ ${pid} == "${started}" ]] || fail "under ${name} ran node as ${pid}, not in its own place, ${started}"
  grep -q "an earlier start" "${log}" || fail "under ${name} moved a short log aside"
  [[ ! -e "${log}.old" ]] || fail "under ${name} kept an old log of a short one"
  [[ ! -e "${work}/slept" ]] || fail "under ${name} waited on a first start"
  words=$(<"${work}/words")
  [[ ${words} == "${expected_words}" ]] || fail "under ${name} gave node ${words}"
  grep -q "node ran" "${log}" || fail "under ${name} didn't send node's output to its log"
  expect TMPDIR "${given}"
  expect XDG_RUNTIME_DIR "${given}"
  # shellcheck disable=SC2016 # the dollar and backquotes are the value, unexpanded
  expect GREETING 'it'"'"'s $HOME, `here`'
  expect HOME /Users/someone
  expect USER someone
  expect LANG C.UTF-8
  expect LC_ALL en_US.UTF-8
  expect MYCELIUM_REALM one
  expect PROXY_ORIGIN https://proxy.example.com
  expect PI_HOST_STOP_USEC 150000000
  expect PI_CODEMODE_MAX_WORKERS 16
  expect UV_THREADPOOL_SIZE 16
  expect_like PATH '^/first/bin:.*:/etc/profiles/per-user/someone/bin:.*:/Users/someone/\.local/bin$'
  expect_like PI_PACKAGE_DIR '^/nix/store/'
  expect_like PI_HOST_PROOF '^/nix/store/'
  # The call probe the host's own restart waits out, as the harness reads it.
  expect_like PI_HOST_RESTART_WAITS_ON '^\[\{"run":\["/nix/store/[^"]*/bin/a-call-is-on"\],"while":"a call is on"\}'
  for loads_code in NODE_OPTIONS NODE_PATH NODE_COMPILE_CACHE LD_PRELOAD LD_AUDIT \
    LD_LIBRARY_PATH OPENSSL_CONF OPENSSL_MODULES; do
    if grep -qz "^${loads_code}=" "${work}/environment"; then
      fail "under ${name} passed on a ${loads_code}"
    fi
  done

  # A log past 10 MB is moved aside whole before the host writes to it.
  fresh
  mkdir -p "${logs}"
  head -c 10485761 /dev/zero >"${log}"
  run GIVEN_TEMPORARY="${given}" || fail "under ${name} failed with a long log"
  [[ $(stat -c %s "${log}.old") == 10485761 ]] || fail "under ${name} didn't move a log past 10 MB aside"
  [[ $(stat -c %s "${log}") -lt 10485760 ]] || fail "under ${name} kept writing to a log past 10 MB"

  # Three starts in the last ten minutes: the fourth says so and waits ten
  # minutes first, then starts; starts an hour old don't hold it.
  fresh
  mkdir -p "${logs}"
  now=$(date +%s)
  printf '%s\n' $((now - 300)) $((now - 200)) $((now - 100)) >"${log}.starts"
  run GIVEN_TEMPORARY="${given}" || fail "under ${name} failed after three quick starts"
  slept=$(slept)
  [[ ${slept} == 600 ]] || fail "under ${name} slept '${slept}', not ten minutes, after three quick starts"
  grep -q "started three times in ten minutes" "${log}" ||
    fail "under ${name} didn't say why it waited"
  [[ -e "${work}/words" ]] || fail "under ${name} never started after waiting"
  [[ $(wc -l <"${log}.starts") == 3 ]] || fail "under ${name} kept $(wc -l <"${log}.starts") starts, not 3"
  printf '%s\n' $((now - 3700)) $((now - 3650)) $((now - 3600)) >"${log}.starts"
  run GIVEN_TEMPORARY="${given}" || fail "under ${name} failed after three old starts"
  [[ ! -e "${work}/slept" ]] || fail "under ${name} waited after starts an hour old"
done
