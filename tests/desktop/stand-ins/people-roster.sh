# shellcheck shell=bash
# Stands in for a People source that knows a crowd, as Tuple's knows every
# contact, run by the people daemon on People's line protocol
# (modules/people): it says 8,000 people unasked, each "Crowd Person N"
# at crowdN@crowd.test, none listed, so the daemon holds them all and
# the shell, which reads only those listed and found, holds none until a
# search finds one. It reads what the daemon asks to its end.
echo '{"roster":true}'
for i in $(seq 8000); do
  printf '{"email":"crowd%s@crowd.test","id":%s,"name":"Crowd Person %s","status":"unknown","favourite":false,"recent":false}\n' \
    "$i" "$i" "$i"
done
while read -r _; do :; done
