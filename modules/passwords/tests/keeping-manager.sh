# shellcheck shell=bash
# A stand-in password manager on Passwords' line protocol, for the tests
# and the completion check: it answers at once, knows GitHub, a bank card
# and a note, gives GitHub in full with its password and one-time seed left
# on, as op does, and logs each ask to the file it's given, so a test can
# say what was asked of it. Its secrets are made up and look it.
log=$1

# GitHub as a list gives it, then its details as a get adds them.
github='{"id":"gh","title":"GitHub","category":"login","version":1,"username":"ada",'
github+='"websites":[{"address":"https://github.com","primary":true}]'
details='"details":['
details+='{"id":"username","label":"username","type":"text","purpose":"username","value":"ada"},'
details+='{"id":"password","label":"password","type":"hidden","purpose":"password",'
details+='"value":"hunter2"},'
details+='{"id":"otp","label":"one-time password","type":"code",'
details+='"value":"otpauth://totp/x?secret=SEED"}]'
card='{"id":"card","title":"Bank Card","category":"card","version":1}'
note='{"id":"note","title":"Door Codes","category":"note","version":1,"tags":["home"]}'

echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s},%s,%s]}\n' "$id" "$github" "$card" "$note" ;;
    get) printf '{"id":%s,"entry":%s,%s}}\n' "$id" "$github" "$details" ;;
    reveal) printf '{"id":%s,"value":"hunter2"}\n' "$id" ;;
    code) printf '{"id":%s,"code":"123456","period":30,"left":12}\n' "$id" ;;
    let-in) printf '{"id":%s}\n' "$id" ;;
  esac
done
