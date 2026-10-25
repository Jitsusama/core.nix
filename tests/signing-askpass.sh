# shellcheck shell=bash
# Answers the agent's prompts for tests/signing-vm-test.py, as Joel does
# through the themed ones, and writes down each one it was asked in his
# runtime folder, which tests/signing.nix puts in place of the marker with
# his PIN. It refuses confirmations while a refuse file is there, and holds a
# confirmation unanswered while a hold file is, as Joel does when he's away.
runtime=@runtime@
pin=@pin@
echo "${SSH_ASKPASS_PROMPT:-pin}" >> "$runtime/asked"
if [ "${SSH_ASKPASS_PROMPT:-}" = confirm ]; then
  while [ -e "$runtime/hold" ]; do
    sleep 0.2
  done
  [ ! -e "$runtime/refuse" ]
else
  echo "$pin"
fi
