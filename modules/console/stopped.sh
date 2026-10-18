# Says on the console what stopped the boot, before a failed boot's shell
# asks for root, which is all there is to read on a machine whose root is
# locked: each unit that failed, the last it said, and the way out. A shell
# asked for with nothing failed says nothing.
# The header written above this (./nixos.nix) gives each role its console
# colour and the mark for a failure its stand-in.

mapfile -t failed < <(systemctl list-units --failed --plain --no-legend --full | cut -d' ' -f1)
((${#failed[@]})) || exit 0

width=$({ stty size 2>/dev/null || echo; } | cut -s -d' ' -f2)
((width > 0)) || width=80

# A piece of text in a role's colour.
paint() {
  printf '\e[%sm%s\e[0m' "$1" "$2"
}

# The last a unit's own programs said, or, when they said nothing, as when
# one was killed, the last systemd said of it.
last() {
  local own
  own=$(journalctl --boot "_SYSTEMD_UNIT=$1" --lines=3 --output=cat --no-pager --quiet || true)
  if [[ -n $own ]]; then
    printf '%s\n' "$own"
  else
    journalctl --boot --unit="$1" --lines=1 --output=cat --no-pager --quiet || true
  fi
}

# The units, each a step in from the edge with what it said under it, kept
# to a screen a person can read before the shell's own words.
margin='  '
under="$margin   "
shown=0
for unit in "${failed[@]}"; do
  echo
  if ((shown == 4)); then
    echo "$under$(paint "$muted" "and $((${#failed[@]} - shown)) more")"
    break
  fi
  about=$(systemctl show --property=Description --value -- "$unit")
  # A unit with no description is described by its name, and one too long
  # to sit beside it goes, since the name says which it is.
  line="$margin$(paint "$alert" "$mark")  $(paint "$strong" "$unit")"
  if [[ $about != "$unit" ]] && ((${#under} + ${#unit} + 2 + ${#about} <= width)); then
    line+="  $(paint "$text" "$about")"
  fi
  echo "$line"
  # What it said, wrapped between words to the console's width, each line
  # without the space it broke at.
  while IFS= read -r said; do
    said=${said%"${said##*[! ]}"}
    echo "$under$(paint "$muted" "$said")"
  done < <(last "$unit" | tr -d '\000-\010\013-\037\177' | fold -s -w $((width - ${#under})))
  shown=$((shown + 1))
done
echo
echo "$under$(paint "$strong" Ctrl+Alt+Delete) $(paint "$text" "restarts; hold") $(paint "$strong" Space) $(paint "$text" "as it starts for an older generation.")"
echo
