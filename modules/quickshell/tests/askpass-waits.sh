# shellcheck shell=bash
# How the askpass waits on the shell (modules/quickshell/home.nix), and
# whom it asks for, run with bash, the built askpass and ssh-tpm-agent's
# gate-cases.txt as its arguments. A stand-in for qs plays a shell that
# answers, one that restarts and so forgets what it was asking, and one
# that's gone. A copy of the askpass runs with the stand-in first on its
# PATH, so no real shell is asked, and reads its caller's cgroup and its
# parents' from a folder of this test's, so each case says where the
# program asking and each of its parents run. Its gate is a file of this
# test's, gone unless a case writes it, so it judges by the gate it was
# built with.
set -u
here=$PWD
mkdir -p bin run
export XDG_RUNTIME_DIR=$here/run STATE=$here
while IFS= read -r line; do
  # The askpass's own line, $PATH and all, as it's written in it.
  # shellcheck disable=SC2016
  if [[ $line == 'export PATH="'*':$PATH"' ]]; then
    line="export PATH=\"$here/bin:${line#export PATH=\"}"
  fi
  [[ $line == proc=/proc ]] && line="proc=$here/proc"
  [[ $line == SSH_ASKPASS_GATE=* ]] && line="SSH_ASKPASS_GATE=$here/gate"
  printf '%s\n' "$line"
done < "$1" > askpass
chmod +x askpass
grep -q "^export PATH=\"$here/bin:" askpass || {
  echo "the askpass no longer sets its PATH as this expects"
  exit 1
}
grep -q "^proc=$here/proc$" askpass || {
  echo "the askpass no longer names /proc as this expects"
  exit 1
}
if ! grep -q "^SSH_ASKPASS_GATE=$here/gate$" askpass ||
  ! grep -q '^SSH_ASKPASS_GATE_BUILT=' askpass; then
  echo "the askpass no longer names its gates as this expects"
  exit 1
fi

# The askpass's caller is this shell: its words as the kernel has them, and
# a cgroup each case names, Joel's terminal's unless it says otherwise,
# then the cgroup of each parent it names after, made up as processes
# numbered from 5000001, past any pid the kernel gives, so none is this
# shell's. The last one's parent is init, as the gate reads a chain to.
mkdir -p "proc/$$"
cp "/proc/$$/cmdline" "proc/$$/cmdline"
user=/user.slice/user-1000.slice/user@1000.service
from() {
  local pid=$$ parent=5000000 place
  rm -rf proc/50000*
  for place in "$@"; do
    parent=$((parent + 1))
    mkdir -p "proc/$pid"
    printf '0::%s\n' "$place" > "proc/$pid/cgroup"
    printf 'Name:\tx\nPPid:\t%s\n' "$parent" > "proc/$pid/status"
    last=$pid
    pid=$parent
  done
  printf 'Name:\tx\nPPid:\t1\n' > "proc/$last/status"
}
# The chain from, with its last process's parent one that's gone.
cut() {
  from "$@"
  printf 'Name:\tx\nPPid:\t4999999\n' > "proc/$last/status"
}

# logger, saying what the askpass journalled.
cat > bin/logger << EOF
#!$BASH
echo "\$*" >> "\$STATE/logged"
EOF
chmod +x bin/logger
from "$user/jobs.slice/jobs-awaited.slice/kitty-1-2.scope"

# qs ipc call askpass <what> <args...>, answered as the shell would, or not
# at all while it's down.
cat > bin/qs << EOF
#!$BASH
what=\$4
echo "\$what \${*:5}" >> "\$STATE/said"
[ "\$(cat "\$STATE/mode")" = up ] || exit 1
case \$what in
ask) echo "\${*: -1}" > "\$STATE/has" ;;
waiting) [ -s "\$STATE/has" ] && echo true || echo false ;;
withdraw) : > "\$STATE/has" ;;
esac
EOF
chmod +x bin/qs

failed=0
# Runs the command after the name, and says whether it held.
check() {
  local name=$1
  shift
  if "$@"; then echo "ok: $name"; else
    echo "FAILED: $name"
    failed=1
  fi
}
# Asks for kind, and waits for the shell to have the ask.
start() {
  : > said
  : > has
  echo up > mode
  SSH_ASKPASS_PROMPT=$1 ./askpass "Allow use of key?" > out 2> err &
  asking=$!
  for _ in $(seq 50); do
    [ -s has ] && break
    sleep 0.1
  done
  reply=$(cat has)
}
ended() {
  wait "$asking"
  echo $? > status
}

# What a check holds to, each run by check by its name, which shellcheck
# can't follow.
# shellcheck disable=SC2329
{
  exited() { [ "$(cat status)" = "$1" ]; }
  printed() { [ "$(cat out)" = "$1" ]; }
  asked() { [ "$(grep -c '^ask ' said)" = "$1" ]; }
  # What it said instead goes with a failure, as the failure's reason.
  complained() { grep -qF -- "$1" err || { cat err; false; }; }
  placed() { grep -qE ' (runs in|was started from) /' err || { cat err; false; }; }
  folder_gone() { ! compgen -G 'run/*askpass*' > /dev/null; }
  between() { [ "$1" -ge "$2" ] && [ "$1" -le "$3" ]; }
}

start secret
printf '2468\n' > "$reply/answer"
ended
check "an answer is said" exited 0
check "as it was typed" printed 2468
check "and asked for once" asked 1
check "and its folder goes" folder_gone

start confirm
: > has
sleep 4
check "a restarted shell is asked again" asked 2
printf 'yes\n' > "$reply/answer"
ended
check "and its yes is taken" exited 0

# Answered between one look and the next, it's never asked again.
start confirm
sleep 2.5
printf 'yes\n' > "$reply/answer.part"
: > has
mv "$reply/answer.part" "$reply/answer"
ended
check "an ask answered as it's looked at isn't asked again" asked 1
check "and its answer is taken" exited 0

start confirm
echo down > mode
began=$SECONDS
ended
took=$((SECONDS - began))
check "a shell that's gone is refused" exited 1
check "saying it went away" complained "went away"
check "after about 20 seconds (it took $took)" between "$took" 18 30
check "and its folder goes" folder_gone

start confirm
echo down > mode
: > has
sleep 7
echo up > mode
sleep 4
check "a shell back within that is asked again" asked 2
printf 'yes\n' > "$reply/answer"
ended
check "and its answer taken" exited 0

# Asked for its help, or given anything but one prompt, it puts nothing on
# the screen.
: > said
echo up > mode
./askpass --help > out 2> err
echo $? > status
check "--help is answered" exited 0
check "opening with its name" grep -q '^ssh-askpass ' out
check "and asks nothing" asked 0
for words in "" "one two"; do
  # Each word stays its own, and none is none.
  # shellcheck disable=SC2086
  ./askpass $words > out 2> err
  echo $? > status
  check "'$words' is refused as asked wrongly" exited 64
  check "and asks nothing" asked 0
done

# Asked for by what Joel runs, it asks; for agents' work, judged by its
# cgroup or a parent's as ssh-tpm-agent's gate judges them, it refuses
# without a word to the shell, saying where the asker ran, there and in
# the journal. An ask that never comes fails saying why it was refused.
# A refusal comes at once, so one held past 3 seconds is stopped and
# fails by its name; it's stopped from here rather than by timeout, so its
# parent is still this shell, whose cgroup each case set.
asks() {
  start confirm
  if [ -z "$reply" ]; then
    kill "$asking" 2> /dev/null
    wait "$asking" 2> /dev/null
    echo "FAILED: $1 is asked for, but it wasn't:"
    cat err
    failed=1
    return
  fi
  printf 'yes\n' > "$reply/answer"
  ended
  check "$1 is asked for" exited 0
  check "and asked once" asked 1
}
refuses() {
  : > said
  : > logged
  SSH_ASKPASS_PROMPT=confirm "${2:-./askpass}" \
    "Are you sure you want to continue connecting?" > out 2> err < /dev/null &
  local refusing=$!
  { sleep 3 && kill "$refusing"; } 2> /dev/null &
  local watching=$!
  wait "$refusing"
  echo $? > status
  kill "$watching" 2> /dev/null
  wait "$watching" 2> /dev/null
  check "$1 is refused" exited 1
  check "and asks nothing" asked 0
  check "and journals it as a warning" grep -q '^-p user.warning -t ssh-askpass -- refused to ask Joel, as ' logged
}
for place in \
  "$user/app.slice/ssh-tpm-agent.service" \
  "$user/app.slice/linux-id.service" \
  "$user/session.slice/niri.service" \
  "$user/app.slice/app-niri-nvim-1.scope"; do
  from "$place" "$user/init.scope"
  asks "one in $place"
done

# Each case the gate must decide, as the gate decides it, but for those
# in background.slice, which only the askpass refuses.
number=0 decided=0
while read -r verdict peer parents <&4; do
  number=$((number + 1))
  case $verdict in
    serve | refuse) ;;
    '' | '#'*) continue ;;
    *)
      echo "FAILED: the gate's case on line $number says $verdict, neither serve nor refuse"
      failed=1
      continue
      ;;
  esac
  decided=$((decided + 1))
  # Each parent is a word of its own.
  # shellcheck disable=SC2086
  from "$peer" $parents
  if [ "$verdict" = serve ] && [[ "$peer $parents" != */background.slice/* ]]; then
    asks "the gate's case on line $number"
  else
    refuses "the gate's case on line $number"
    check "for where it ran" placed
  fi
done 4< "$2"
check "having decided the gate's cases ($decided)" between "$decided" 40 1000

# nix-offload's serving units sign in with keys the gate never sees, and
# units in background.slice are Joel's daemons or ones agents name, so
# only the askpass refuses them, as it does a cgroup read as the root,
# which only a namespace shows.
from "$user/offload.slice/nix-offload-vm-8.service" "$user/init.scope"
refuses "nix-offload's serving unit"
check "saying where it runs" complained "runs in $user/offload.slice/nix-offload-vm-8.service"
from "$user/background.slice/queue-isxeg8-1.service" "$user/init.scope"
refuses "a unit an agent names in background.slice"
from "/" "/"
refuses "one in a cgroup namespace of its own"
from "$user/app.slice/run-u1.scope" "$user/jobs.slice/pi-spawner-1-2.scope" "$user/init.scope"
refuses "a command moved out of its spawner's scope"
check "saying where it was started" complained "was started from $user/jobs.slice/pi-spawner-1-2.scope"
from "$user/agents.slice/kitty-1-2.scope/app.slice/kitty-3-4.scope"
refuses "a cgroup named as Joel's inside agents' place"

# A chain it can't read to init is refused, as the gate refuses it.
cut "$user/app.slice/kitty-2210-1.scope" "$user/app.slice/app-niri-kitty-1.scope"
refuses "one whose parent is gone as it's read"
check "saying so" complained "has a parent (pid 4999999) whose place can't be read"
long=()
for _ in $(seq 33); do long+=("$user/app.slice/kitty-2210-1.scope"); done
from "${long[@]}"
refuses "one with more than 32 parents"
check "saying so" complained "has more than 32 parents"
from "$user/app.slice/kitty-2210-1.scope"
printf '1:name=x:/x\n' > "proc/$$/cgroup"
refuses "one with no unified cgroup"
check "saying so" complained "has no unified cgroup"
rm "proc/$$/cgroup"
refuses "one whose cgroup can't be read"
check "saying it's gone" complained "is gone, or its cgroup can't be read"

# The agent's own gate file, where there is one, is the gate: a line that
# allows decides as it does there, and one opening with watch asks for
# what it would refuse, journalling it.
from "$user/agents.slice/kitty-1-2.scope" "$user/init.scope"
printf '%s\n' 'allow /agents\.slice/kitty-1-2\.scope$' 'refuse /agents\.slice/' > gate
asks "an asker a line of the gate's own file allows"
printf '%s\n' '  # watching' '' 'watch' 'refuse /agents\.slice/' > gate
: > logged
asks "an asker a watching gate would refuse"
check "journalling that it would have" grep -q 'would have refused to ask Joel, as ' logged
for broken in 'refuse (' 'deny /agents\.slice/' 'allow /x' 'watch /x'; do
  printf '%s\n' "$broken" > gate
  from "$user/app.slice/kitty-2210-1.scope" "$user/init.scope"
  refuses "Joel's terminal under a gate saying '$broken'"
  check "saying its gate can't be judged by" complained "gate ($here/gate) can't be judged by"
done
rm gate

# With no gate it asks for no one, Joel's own terminal included.
sed 's|^\(SSH_ASKPASS_GATE\(_BUILT\)\{0,1\}\)=.*|\1=/nowhere|' askpass > ungated
chmod +x ungated
refuses "one asking while no gate can be read" ./ungated
check "saying so" complained "gate (/nowhere) can't be judged by: it can't be read"
exit "$failed"