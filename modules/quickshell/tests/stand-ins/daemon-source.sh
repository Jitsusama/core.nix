# shellcheck shell=bash
# Stands in for one of People's sources, run by the people daemon on its
# line protocol (modules/people): an address is a step-in and `?ADDRESS`
# an ask ahead, and each is answered with who it is and which way it was
# asked, so tests/daemon.qml can tell a get's answer from a watch's. It
# knows Ada and Grace, and nobody else.
while read -r line; do
  case $line in
    '?'*) address=${line#?} way=ahead ;;
    *) address=$line way=step ;;
  esac
  case $address in
    ada@example.com | grace@example.com)
      printf '{"email":"%s","name":"%s","way":"%s"}\n' "$address" "${address%@*}" "$way"
      ;;
    *) printf '{"email":"%s","id":null}\n' "$address" ;;
  esac
done
