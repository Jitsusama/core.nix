# shellcheck shell=bash
# A stand-in for slack-people, the people daemon's source for Slack, for
# tests/people.qml: asked about Zoë, as a step-in or ahead, it says Slack
# found her, active, in the fields slack-people says them in. It knows
# nobody else and says nothing of anyone else asked, so the steps that
# read the order of someone's acts see no Write in Slack among them.
while read -r line; do
  if [ "${line#\?}" = "zoe@example.com" ]; then
    echo '{"email":"zoe@example.com","id":"U9","team":"T1","name":"Zoë \"Zed\" Quill",'\
'"presence":"active","status":"","emoji":"","zone":"","handle":"zoe","title":""}'
  fi
done
