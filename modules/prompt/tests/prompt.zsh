# The prompt gives way: from 120 columns down to 16, in a rebase with
# everything else showing too, the folder Joel is in and the glyph stay,
# room is left to type until there's nothing left to give, and what gives
# way goes without a trace. Run as zsh -f -i -c 'source this
# prompt.zsh folder branch', the folder being two below the root of a
# repository on that branch.
setopt extended_glob
source $1
cd $2

fail() {
  print -u2 -- "$@"
  exit 1
}

# What a terminal would show of a prompt string.
plain() {
  local drawn=${(%)1}
  REPLY=${drawn//$'\e'\[[0-9;]#m/}
}

IN_NIX_SHELL=impure
_prompt_precmd
_prompt_state[unstaged]=1 _prompt_state[untracked]=1
_prompt_state[ahead]=2 _prompt_state[behind]=1 _prompt_state[jobs]=1
_prompt_took=12.4s
leaf=${PWD:t}
branch=${_prompt_state[branch]}
more=${_prompt_glyph[more]}
[[ ${_prompt_state[op]} == rebase && ${_prompt_state[at]} == 2 ]] ||
  fail "The rebase under way wasn't read: ${(kv)_prompt_state}"
[[ $branch == $3 ]] || fail "HEAD was read as $branch, not $3."

for COLUMNS in {120..16}; do
  _prompt_lay
  plain $_prompt_left
  left=$REPLY
  plain $_prompt_right
  right=$REPLY
  [[ $left == *$leaf* ]] || fail "$COLUMNS columns lost the folder: $left"
  [[ $left == *${_prompt_glyph[prompt]}' ' ]] || fail "$COLUMNS columns lost the glyph: $left"
  if (( _prompt_level <= ${#_prompt_steps} )); then
    (( ${(m)#left} + 8 + (${#right} ? ${(m)#right} + 2 : 0) <= COLUMNS )) ||
      fail "$COLUMNS columns leave no room to type: $left$right"
  fi
  # What gives way leaves no mark behind: none before the glyph once the
  # right side has gone, and none at the end of the right side. A branch
  # cut short keeps the mark inside its name.
  [[ $left != *"$more ${_prompt_glyph[prompt]} " ]] ||
    fail "$COLUMNS columns mark the right side's going: $left"
  [[ $right != *$more ]] || fail "$COLUMNS columns mark what gave way: $right"
done
(( COLUMNS = 120 ))
_prompt_lay
plain $_prompt_right
[[ $REPLY == *$branch* && $REPLY == *12.4s* ]] || fail "120 columns don't show it all: $REPLY"
# Git's notation after the branch: what the tree holds, then the upstream.
[[ $REPLY == *"$branch *? ↑2↓1" ]] ||
  fail "The branch isn't followed by git's notation: $REPLY"
# A tree clean and in step adds nothing after the branch.
_prompt_clean
_prompt_lay
plain $_prompt_right
[[ $REPLY == *$branch ]] || fail "A clean tree has something after its branch: $REPLY"
