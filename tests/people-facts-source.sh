# shellcheck shell=bash
# A stand-in people daemon source for tests/people.qml that says Ada's
# facts unasked, as a roster does, in the ready forms slack-people says
# them: her team, saying it's People's team so her drawer heads with it,
# and her time zone left to the shell's clock to word. It then reads what
# it's asked and says nothing more.
echo '{"roster":true}'
echo '{"email":"ada@example.com","facts":[{"name":"Team","said":"Shell","wordsId":"people:team"},'\
'{"name":"Time Zone","zone":"Asia/Kolkata","as":"zone"}]}'
cat > /dev/null
