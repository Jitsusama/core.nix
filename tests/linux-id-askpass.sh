# shellcheck shell=bash
# Answers as Joel does through the shell's prompt, for tests/linux-id.nix, and writes down
# each question in his runtime folder, which tests/linux-id.nix puts in place of the marker,
# until a refuse file there tells it to refuse.
runtime=@runtime@
echo "${SSH_ASKPASS_PROMPT:-secret} $1" >> "$runtime/asked"
[ ! -e "$runtime/refuse" ]
