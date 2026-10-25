# The prompt gives way: from 120 columns down to 16, in a rebase with
# everything else showing too, the folder Joel is in and the glyph stay,
# room is left to type until there's nothing left to give, and the more mark
# shows whenever something is hidden. Run as zsh -f -i -c 'source this
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
_prompt_state[unsaved]=1 _prompt_state[ahead]=1 _prompt_state[jobs]=1
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
  # The mark stands where the hiding happened: on the right while it's
  # drawn, and just before the glyph once it's gone.
  if [[ -z $right ]]; then
    [[ $left == *"$more ${_prompt_glyph[prompt]} " ]] ||
      fail "$COLUMNS columns hide the right side unmarked: $left"
  elif [[ $right != *$branch* || $right != *12.4s* || $right != *${_prompt_glyph[shell]}* ]]; then
    [[ $right == *$more* ]] || fail "$COLUMNS columns hide something unmarked: $right"
  fi
done
(( COLUMNS = 120 ))
_prompt_lay
plain $_prompt_right
[[ $REPLY == *$branch* && $REPLY == *12.4s* ]] || fail "120 columns don't show it all: $REPLY"
