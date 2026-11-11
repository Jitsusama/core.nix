# shellcheck shell=bash
# A password manager on Passwords' line protocol standing in for 1Password's,
# for tests/passwords-on-niri-test.py: it never runs op, and the only secret it
# knows is a stand-in's. It starts signed out, as 1Password's does, and refuses
# anything asked before it's let in, as op does when Joel dismisses
# 1Password's prompt. The first let-in is refused too, as a dismissed prompt's
# is, so the test can say the launcher tells why; the second lets it in, and it
# knows GitHub and a bank card. Each ask is logged to the file it's given, or
# else to $PASSWORDS_ASKED, as the shell's passwords daemon runs it with the
# test's environment, so the test can say what was asked and when.
#
# usage: passwords-stand-in.sh [LOG]

log=${1:-$PASSWORDS_ASKED}
let_in=false
asked_in=0
locked='"1Password asks you to let the launcher in"'
github='{"id":"gh","title":"GitHub","category":"login","version":3,"username":"ada",'
github+='"favourite":true,"tags":["work"],"collection":{"id":"v1","name":"Private"},'
github+='"websites":[{"address":"https://github.com/login","label":"website","primary":true}],'
github+='"created":1700000000000,"changed":1794000000000,"own":{"account":"ACCT1"}'
card='{"id":"card","title":"Bank Card","category":"card","version":1,'
card+='"collection":{"id":"v1","name":"Private"},"changed":1793900000000,"own":{}}'
# GitHub read in full, as 1Password's manager gives it: no hidden detail's value.
details='"details":[{"id":"username","label":"username","type":"text","hidden":false,'
details+='"purpose":"username","empty":false,"value":"ada"},{"id":"password",'
details+='"label":"password","type":"hidden","hidden":true,"purpose":"password",'
details+='"empty":false,"strength":"fantastic"},{"id":"TOTP_1","label":"one-time password",'
details+='"type":"code","hidden":true,"section":"Security","empty":false}]'

echo "{\"state\":\"signed-out\",\"reason\":$locked}"
while read -r line; do
  printf '%s\n' "$line" >>"$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  entry=
  [[ $line =~ \"entry\":\"([a-z]+)\" ]] && entry=${BASH_REMATCH[1]}
  if [[ $ask == let-in ]] && ((asked_in++ == 0)); then
    refused="{\"id\":$id,\"error\":{\"code\":\"signed-out\",\"message\":$locked},"
    echo "$refused\"state\":\"signed-out\",\"reason\":$locked}"
    continue
  fi
  if [[ $ask == let-in ]]; then
    let_in=true
    echo "{\"id\":$id,\"state\":\"answering\"}"
    continue
  fi
  if [[ $let_in != true ]]; then
    refused="{\"id\":$id,\"error\":{\"code\":\"signed-out\",\"message\":$locked},"
    echo "$refused\"state\":\"signed-out\",\"reason\":$locked}"
    continue
  fi
  case $ask in
  list) echo "{\"id\":$id,\"state\":\"answering\",\"entries\":[$github},$card]}" ;;
  get)
    if [[ $entry == card ]]; then
      echo "{\"id\":$id,\"state\":\"answering\",\"entry\":${card%\}},\"details\":[]}}"
    else
      echo "{\"id\":$id,\"state\":\"answering\",\"entry\":$github,$details}}"
    fi
    ;;
  reveal) echo "{\"id\":$id,\"value\":\"stand-in-password\"}" ;;
  code) echo "{\"id\":$id,\"code\":\"246810\",\"period\":30,\"left\":20}" ;;
  *) echo "{\"id\":$id,\"error\":{\"code\":\"refused\",\"message\":\"no such ask\"}}" ;;
  esac
done
