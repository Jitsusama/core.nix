# shellcheck shell=bash
# Stands in for 1Password's command, op, as 1Password's manager for
# Passwords runs it (modules/onepassword/manager), since no 1Password
# account can sign in to the VM. It answers as op 2 does, in its JSON, for
# three items: a favourite login with a weak password, a website sent in the
# clear and a one-time code, so its drawer warns; a login changed lately;
# and a card. Every secret it knows is a stand-in's, and its times are a few
# days before now, so how long ago reads the same on any day.
ago() {
  date -u -d "-$1 days" +%Y-%m-%dT%H:%M:%SZ
}
console='{"id":"console","title":"Lighthouse Console","version":4,"category":"LOGIN",'
console+='"favorite":true,"tags":["keeping"],"additional_information":"keeper",'
console+='"vault":{"id":"keeping","name":"Keeping"},'
console+='"urls":[{"primary":true,"href":"http://console.lighthouse.test"}],'
console+="\"created_at\":\"$(ago 40)\",\"updated_at\":\"$(ago 3)\""
github='{"id":"github","title":"GitHub","version":2,"category":"LOGIN","favorite":false,'
github+='"tags":[],"additional_information":"ada","vault":{"id":"keeping","name":"Keeping"},'
github+='"urls":[{"primary":true,"href":"https://github.com/login"}],'
github+="\"created_at\":\"$(ago 90)\",\"updated_at\":\"$(ago 1)\""
card='{"id":"card","title":"Bank Card","version":1,"category":"CREDIT_CARD",'
card+='"favorite":false,"tags":[],"vault":{"id":"shared","name":"Shared"},'
card+="\"created_at\":\"$(ago 200)\",\"updated_at\":\"$(ago 5)\""
# The console read in full: its username, its password and its one-time
# code's seed, each a stand-in's, as op gives them, values and all.
fields='"sections":[{"id":"security","label":"Security"}],"fields":['
fields+='{"id":"username","type":"STRING","purpose":"USERNAME","label":"username",'
fields+='"value":"keeper"},{"id":"password","type":"CONCEALED","purpose":"PASSWORD",'
fields+='"label":"password","value":"stand-in-password",'
fields+='"password_details":{"strength":"WEAK"}},{"id":"TOTP_1","type":"OTP",'
fields+='"label":"one-time password","section":{"id":"security"},'
fields+='"value":"otpauth://totp/stand-in?secret=STANDINSTANDIN&period=30"}]'

case "$1 $2" in
"vault list") echo '[{"id":"keeping","name":"Keeping"},{"id":"shared","name":"Shared"}]' ;;
"account get") echo '{"id":"STANDIN","name":"Stand-in","domain":"my.1password.com"}' ;;
"item list") echo "[$console},$github},$card}]" ;;
"item get")
  if [ "$4" = "--otp" ]; then
    echo 246810
  elif [ "$3" = console ]; then
    echo "$console,$fields}"
  elif [ "$3" = github ]; then
    echo "$github,\"fields\":[]}"
  else
    echo "$card,\"fields\":[]}"
  fi
  ;;
*)
  echo "[ERROR] the stand-in for op can't answer $*" >&2
  exit 1
  ;;
esac
