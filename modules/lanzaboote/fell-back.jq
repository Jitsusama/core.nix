# From bootctl's list of entries, what to say when the generation booted
# isn't the newest because a newer one ran out of its $tries, or nothing.
def generation: capture("^nixos-generation-(?<n>[0-9]+)").n | tonumber;
([.[] | select(.isSelected) | .id | generation?] | first) as $booted
| select($booted != null)
| [.[] | select(.triesLeft == 0) | .id | generation? | select(. > $booted)]
| max // empty
| "Generation \(.) failed to boot \($tries) times, so this is \($booted)."
