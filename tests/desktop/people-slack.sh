# shellcheck shell=bash
# Stands in for slack-people, Slack's reader, which the people daemon runs
# on People's line protocol (modules/slack/src/main.rs): an address a line
# is a step-in, `?ADDRESS` an ask ahead. Until a test writes the runtime
# folder's slack-signed-in it has no sign-in, and says so to each line as
# the real one does, so no other test sees Slack; once it has, it knows
# Crowd Person 1234, active, and nobody else. Each line asked of it goes
# to slack-asked, so a test sees the shell's step-in came through the
# daemon.
runtime=${XDG_RUNTIME_DIR:?}
while read -r line; do
  echo "$line" >>"$runtime/slack-asked"
  address=${line#\?}
  if [ ! -e "$runtime/slack-signed-in" ]; then
    echo '{"signedIn":false}'
  elif [ "$address" = crowd1234@crowd.test ]; then
    printf '{"email":"%s","id":"U1234","team":"T1","name":"Crowd Person 1234","presence":"active"}\n' \
      "$address"
  else
    printf '{"email":"%s","id":null}\n' "$address"
  fi
done
