# shellcheck shell=bash
# Stands in for slack-people, Slack's reader, which the people daemon runs
# on People's line protocol (modules/slack/src/main.rs): an address a line
# is a step-in, `?ADDRESS` an ask ahead. Until a test writes the runtime
# folder's slack-signed-in it has no sign-in, and says so to each line as
# the real one does, so no other test sees Slack; once it has, it knows
# Crowd Person 1234, active, Write in Slack leading the ways to reach
# them, and nobody else: anyone else is `known` false, and once stepped
# into holds Write in Slack's place, dimmed, saying Slack knows no one
# there, as the real one serves them. Each line asked of it goes to
# slack-asked, so a test sees the shell's step-in came through the daemon.
runtime=${XDG_RUNTIME_DIR:?}
stepped=" "
while read -r line; do
  echo "$line" >>"$runtime/slack-asked"
  address=${line#\?}
  if [ "$address" = "$line" ]; then
    stepped="$stepped$address "
  fi
  if [ ! -e "$runtime/slack-signed-in" ]; then
    echo '{"signedIn":false}'
  elif [ "$address" = crowd1234@crowd.test ]; then
    write='[{"key":"verb:person:slack","open":"slack://user?team=T1&id=U1234","main":true}]'
    printf '{"email":"%s","id":"U1234","team":"T1","name":"Crowd Person 1234",%s,%s}\n' \
      "$address" '"presence":"active"' "\"acts\":$write"
  else
    acts='[]'
    case "$stepped" in
      *" $address "*)
        acts='[{"key":"verb:person:slack","side":"no one at this address","gone":true,'
        acts+='"main":true}]'
        ;;
    esac
    printf '{"email":"%s","id":null,"known":false,"acts":%s}\n' "$address" "$acts"
  fi
done
