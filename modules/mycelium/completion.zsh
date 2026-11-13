#compdef mycelium
# Completes the swarm's teams by the names a person types, after
# `mycelium swarm` and around its acts. Only the node knows the teams, so
# they're asked of it (`mycelium swarm --names`) as the line is completed,
# in the realm the line names; anything else is completed as files.

local -a realm teams acts
local i
for (( i = 2; i < CURRENT; i++ )); do
  case $words[i] in
    --realm) realm=(--realm "$words[i+1]") ;;
    --realm=*) realm=("$words[i]") ;;
  esac
done

local at=${words[(i)swarm]}
if (( at >= CURRENT )); then
  _files
  return
fi

# A team is named before its act or in the one word after it, so the
# first word offers both, the word after an act its team, and the word
# after a team its acts.
acts=(say pause resume)
teams=(${(f)"$(mycelium $realm swarm --names 2>/dev/null)"})
local before=$words[CURRENT-1]
if (( CURRENT == at + 1 )); then
  compadd -X 'a team, or what to do to one' -- $acts $teams
elif (( ${acts[(Ie)$before]} )); then
  (( CURRENT == at + 2 )) && compadd -X 'a team' -- $teams
elif (( CURRENT == at + 2 )) && (( ${teams[(Ie)$before]} )); then
  compadd -X 'what to do to it' -- $acts
fi
