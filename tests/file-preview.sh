#!/usr/bin/env bash
# A stand-in for file-preview, kept (serve, a line asked at a time) or
# asked once (show): it answers each file with a picture's sides and the
# process that answered, so a case sees whether one kept was asked.
answer() {
  printf '{"picture": "%s", "across": 4, "down": 3, "by": %s}\n' "${1#show }" "$$"
}
if [ "$1" = serve ]; then
  while IFS= read -r line; do answer "$line"; done
else
  answer "$*"
fi
