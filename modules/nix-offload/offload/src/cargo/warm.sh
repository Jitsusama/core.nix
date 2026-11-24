# shellcheck shell=sh
# A suite's shell on a lent builder, run as `sh -c WARM nix-offload-warm
# KEY COMMAND...`: it points cargo at a target kept there for KEY, then
# runs COMMAND. Why, and what each fallback keeps, is in cargo.rs's WARM.
key=$1
shift
cache=$HOME/.cache/nix-offload/cargo
root=$cache/$key
room=${NIX_OFFLOAD_WARM_ROOM_KB:-209715200}

# Whether the disk the targets sit on has less free than `room`; a disk
# whose room can't be read isn't short.
short() {
	free=$(df -Pk "$cache" 2>/dev/null | awk 'NR == 2 { print $4 }')
	[ -n "$free" ] && [ "$free" -lt "$room" ]
}

if [ -n "${CARGO_TARGET_DIR-}" ]; then
	echo "nix-offload: the builder names cargo's target itself, so this builds there" >&2
elif ! command -v flock >/dev/null 2>&1; then
	echo "nix-offload: the builder has no flock, so this builds cold" >&2
# The copies kept beside the targets outlive their runs, so the account's
# alone may read them.
elif ! (umask 077 && mkdir -p "$root") 2>/dev/null; then
	echo "nix-offload: no warm target can be made under $cache there, so this builds cold" >&2
else
	slot=
	for n in 0 1 2 3; do
		# `command` keeps a lock that can't be opened from ending the shell,
		# as a failed redirection of `exec` itself does in dash.
		if { command exec 8>"$root/$n.lock"; } 2>/dev/null && flock -n 8; then
			slot=$n
			break
		fi
	done
	if [ -z "$slot" ]; then
		exec 8>&-
		echo "nix-offload: no warm target of $key there is free, so this builds cold" >&2
	else
		# A lock's time says when its target was last taken, so the room
		# rule clears the targets longest unused first.
		touch "$root/$slot.lock"
		if short; then
			# shellcheck disable=SC2045 # ls sorts by age, which a glob can't; keys and slots are names a dash and a dot can't break
			for lock in $(ls -tr "$cache"/*/*.lock 2>/dev/null); do
				short || break
				[ "$lock" = "$root/$slot.lock" ] && continue
				if { command exec 9>>"$lock"; } 2>/dev/null && flock -n 9; then
					idle=${lock%.lock}
					# A target cleared before keeps its lock, so only one still
					# there is said to be cleared.
					if [ -d "$idle" ]; then
						rm -rf "$idle"
						echo "nix-offload: the builder is short of room, so it clears warm target" \
							"${idle#"$cache"/}" >&2
					fi
				fi
				exec 9>&-
			done
			if short; then
				rm -rf "${root:?}/$slot"
				echo "nix-offload: the builder is still short of room, so warm target $slot" \
					"of $key begins afresh" >&2
			fi
		fi
		# A copy's folder is new each run, so cargo keyed each crate outside
		# the run's workspace, as a kit beside it, by a new path and left its
		# last build behind. Moved to a folder each run of this target uses,
		# each build replaces the last; the copy's own cleanup then finds an
		# empty folder. A run outside such a copy stays where it is.
		rest=${PWD#*/nix-offload-changed-*/tree}
		case $rest in
		"$PWD") ;;
		"" | /*)
			top=${PWD%"$rest"}
			rm -rf "${root:?}/$slot/src" 2>/dev/null
			# A src the last run left that can't all go would take the copy
			# inside it, and the suite would run on the old files.
			if [ ! -e "$root/$slot/src" ] && mkdir -p "$root/$slot" &&
				mv "$top" "$root/$slot/src" 2>/dev/null && cd "$root/$slot/src$rest"; then
				echo "nix-offload: the copy runs from warm target $slot's own folder" >&2
			else
				echo "nix-offload: the copy can't move into warm target $slot, so crates" \
					"beside its workspace build at a new path" >&2
			fi
			;;
		# A folder whose name only begins as the copy's does, as tree2, is no copy.
		*) ;;
		esac
		export CARGO_TARGET_DIR="$root/$slot/target" CARGO_BUILD_BUILD_DIR="$root/$slot/build"
		echo "nix-offload: cargo builds in warm target $slot of $key there" >&2
	fi
fi
exec "$@"
