# shellcheck shell=bash # The check runs it with bash.
# Runs the shells' texts that name SSH's agent, zsh's given first and
# bash's second, as Home Manager writes them: zsh keeps the agent it was
# given, and one given none points at the TPM's, while bash's, which the
# login's services run, points them at the TPM's whatever they held.
set -euo pipefail

zsh_text=$1
bash_text=$2
export XDG_RUNTIME_DIR=/run/u
here=/run/u/ssh-tpm-agent.sock
# shellcheck disable=SC2016 # The shells it's given to expand it, after each text.
said='; print -r -- $SSH_AUTH_SOCK'
given=$(SSH_AUTH_SOCK=/given zsh -fc "${zsh_text}${said}")
none=$(env -u SSH_AUTH_SOCK zsh -fc "${zsh_text}${said}")
# shellcheck disable=SC2016 # The shell it's given to expands it, after the text.
service=$(SSH_AUTH_SOCK=/given bash -c "${bash_text}"'; echo "$SSH_AUTH_SOCK"')
if [[ ${given} != /given || ${none} != "${here}" || ${service} != "${here}" ]]; then
  echo "SSH_AUTH_SOCK: zsh given one kept ${given}, zsh given none took ${none}," \
    "and the login's services took ${service}" >&2
  exit 1
fi
