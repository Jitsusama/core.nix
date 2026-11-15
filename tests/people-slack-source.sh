# shellcheck shell=bash
# A stand-in for slack-people, the people daemon's source for Slack, for
# tests/people.qml: asked about Zoë, as a step-in or ahead, it says Slack
# found her, active, in the fields slack-people says them in, with Reply
# in Slack leading her ways, since her message to Joel waits on him, and
# that conversation between them as its ready row. Asked about
# noone@example.com, it says Slack knows nobody by it, with no id and
# `known` false, and once it's been stepped into Write in Slack's place
# held, saying so, ahead too, as slack-people does. Asked about
# unasked@example.com it says nothing, so the place the people daemon
# holds for Write in Slack while it's asked stays. Asked about
# zed@example.com, Zoë's other address, it knows nobody by it but says
# what's between them there: an older note waiting on Joel and a 1:1, so
# a person known by both has what's under each. Anyone else it says
# it knows nobody by, with no act even once stepped into, so the steps
# that read the order of someone's acts see no Write in Slack among them
# once it's answered.
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
    zed@example.com)
      echo '{"email":"zed@example.com","id":null,"known":false,"acts":[],"between":['\
'{"at":1730200000000,"key":"slack:conversation:D8","kind":"wrote","mine":false,'\
'"what":"Slack","waits":{},"row":{"name":"Old note","acts":[]}},'\
'{"at":1730300000000,"key":"slack:huddle:H1","kind":"met","one":true,'\
'"row":{"name":"Coffee"}}]}'
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
    unasked@example.com) ;;
    *)
      echo '{"email":"'"${line#\?}"'","id":null,"known":false,"acts":[]}'
      ;;
  esac
done
