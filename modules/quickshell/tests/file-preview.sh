#!/usr/bin/env bash
# A stand-in for file-preview, kept (serve, a line asked at a time) or
# asked once (show): it answers each file with a picture's sides and the
# process that answered, so a case sees whether one kept was asked. Asked
# as `silent`, it stops before answering, as one that can't start does.
answer() {
  printf '{"picture": "%s", "across": 4, "down": 3, "by": %s}\n' "${1#show }" "$$"
}
if [ "$1" = silent ]; then
  exit 3
elif [ "$1" = serve ]; then
  while IFS= read -r line; do answer "$line"; done
else
  answer "$*"
fi
