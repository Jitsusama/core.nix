# shellcheck shell=bash
# A stand-in for slack-people, the people daemon's source for Slack, for
# tests/people.qml: asked about Zoë, as a step-in or ahead, it says Slack
# found her, active, in the fields slack-people says them in, with Reply
# in Slack leading her ways, since her message to Joel waits on him, and
# that conversation between them as its ready row. Asked about
# noone@example.com, it says Slack knows nobody by it, with no id and
# `known` false, and once it's been stepped into Write in Slack's place
# held, saying so, ahead too, as slack-people does. It says nothing of anyone else asked, so the
# steps that read the order of someone's acts see no Write in Slack among
# them.
stepped=""
while read -r line; do
  case "${line#\?}" in
    zoe@example.com)
      echo '{"email":"zoe@example.com","id":"U9","team":"T1","name":"Zoë \"Zed\" Quill",'\
'"presence":"active","status":"","emoji":"","zone":"","handle":"zoe","title":"",'\
'"acts":[{"key":"verb:person:slack:reply","open":"slack://channel?team=T1&id=D9",'\
'"main":true}],'\
'"between":[{"at":1730312345000,"key":"slack:conversation:D9","kind":"wrote",'\
'"mine":false,"what":"Slack","waits":{},"row":{"name":"Lunch on Friday?",'\
'"glyph":"chat","hue":"program","side":[{"said":"Slack"},{"at":1730312345000,'\
'"as":"ago"}],"acts":[{"key":"verb:slack:open","open":"slack://channel?team=T1&id=D9"}],'\
'"drawer":{"title":"Zoë \"Zed\" Quill","facts":[],"body":"Zoë: Lunch on Friday?"}}}]}'
      ;;
    noone@example.com)
      if [ "${line:0:1}" != "?" ]; then
        stepped=1
      fi
      if [ -z "$stepped" ]; then
        acts='[]'
      else
        acts='[{"key":"verb:person:slack","side":"no one at this address","gone":true,'
        acts+='"main":true}]'
      fi
      echo '{"email":"noone@example.com","id":null,"team":"T1","name":"","presence":"",'\
'"status":"","emoji":"","zone":"","handle":"","known":false,"acts":'"$acts"'}'
      ;;
    *)
      # Silent, as slack-people is while Slack hasn't yet answered.
      ;;
  esac
done
