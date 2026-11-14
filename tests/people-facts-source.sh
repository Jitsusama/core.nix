# shellcheck shell=bash
# A stand-in people daemon source for tests/people.qml that says Ada's
# facts unasked, as a roster does, in the ready forms slack-people says
# them: her team, saying it's People's team so her drawer heads with it,
# and her time zone left to the shell's clock to word. Eli, whom nobody
# lists, has a title, which only a drawer the daemon serves says, and a
# time zone apart from here, which his row says found as listed; Tuple
# lists him before the other strangers, so which of the two sources names
# him first leaves the search's order as it was. It then reads what it's
# asked and says nothing more.
echo '{"roster":true}'
echo '{"email":"ada@example.com","facts":[{"name":"Team","said":"Shell","wordsId":"people:team"},'\
'{"name":"Time Zone","zone":"Asia/Kolkata","as":"zone"}]}'
echo '{"email":"eli@example.com","facts":[{"name":"Title","said":"Designer",'\
'"wordsId":"people:title"},{"name":"Time Zone","zone":"America/Toronto","as":"zone"}]}'
cat > /dev/null
