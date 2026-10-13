# How the askpass waits on the shell (modules/quickshell/home.nix), run with
# bash and the built askpass as its argument. A stand-in for qs plays a
# shell that answers, one that restarts and so forgets what it was asking,
# and one that's gone. A copy of the askpass runs with the stand-in first on
# its PATH, so no real shell is asked.
set -u
here=$PWD
mkdir -p bin run
export XDG_RUNTIME_DIR=$here/run STATE=$here
while IFS= read -r line; do
  if [[ $line == 'export PATH="'*':$PATH"' ]]; then
    line="export PATH=\"$here/bin:${line#export PATH=\"}"
  fi
  printf '%s\n' "$line"
done < "$1" > askpass
chmod +x askpass
grep -q "^export PATH=\"$here/bin:" askpass || {
  echo "the askpass no longer sets its PATH as this expects"
  exit 1
}

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
check() {
  if eval "$2"; then echo "ok: $1"; else
    echo "FAILED: $1"
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
asks() { grep -c '^ask ' said; }
ended() {
  wait "$asking"
  echo $? > exit
}

start secret
printf '2468\n' > "$reply/answer"
ended
check "an answer is said" '[ "$(cat exit)" = 0 ] && [ "$(cat out)" = 2468 ]'
check "and asked for once" '[ "$(asks)" = 1 ]'
check "and its folder goes" '! ls run | grep -q askpass'

start confirm
: > has
sleep 4
check "a restarted shell is asked again" '[ "$(asks)" = 2 ]'
printf 'yes\n' > "$reply/answer"
ended
check "and its yes is taken" '[ "$(cat exit)" = 0 ]'

# Answered between one look and the next, it's never asked again.
start confirm
sleep 2.5
printf 'yes\n' > "$reply/answer.part"
: > has
mv "$reply/answer.part" "$reply/answer"
ended
check "an ask answered as it's looked at isn't asked again" \
  '[ "$(asks)" = 1 ] && [ "$(cat exit)" = 0 ]'

start confirm
echo down > mode
began=$SECONDS
ended
took=$((SECONDS - began))
check "a shell that's gone is refused" '[ "$(cat exit)" = 1 ] && grep -q "went away" err'
check "after about 20 seconds (it took $took)" '[ "$took" -ge 18 ] && [ "$took" -le 30 ]'
check "and its folder goes" '! ls run | grep -q askpass'

start confirm
echo down > mode
: > has
sleep 7
echo up > mode
sleep 4
check "a shell back within that is asked again" '[ "$(asks)" = 2 ]'
printf 'yes\n' > "$reply/answer"
ended
check "and its answer taken" '[ "$(cat exit)" = 0 ]'
exit $failed
