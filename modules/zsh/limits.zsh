# How many files the shell's commands, and its prompt's git-engine, may
# hold open. Left to systemd's default, a Linux desktop session gives each
# program 1024, and a Mac 256, and a git fetch in a large repository whose
# refs are kept as tables holds every worktree's tables open beside its
# packs, well past that, then calls a sound object corrupt when it can't
# open one. So the shell raises its soft limit to 8192, or to the hard
# limit where that's lower. A soft limit already higher is kept, and the
# hard limit is left where the session put it, so nothing the session
# allows is taken away. Where the session already gives more, as core's
# slices module does for every unit the account starts (its hard limit as
# its soft one), the shell keeps it. The limits are read in subshells,
# which start no program.
() {
  local soft=$(ulimit -Sn) hard=$(ulimit -Hn) want=8192
  [[ $soft == unlimited ]] && return
  (( soft >= want )) && return
  [[ $hard != unlimited ]] && (( hard < want )) && want=$hard
  ulimit -Sn $want
}
