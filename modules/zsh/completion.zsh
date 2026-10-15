# Completion, from a dump kept while the folders it was made from still hold
# what they held. Each folder on fpath is followed to where it really is, and
# a switch that changes what one holds points it somewhere new, so the
# folders' real places say when the dump is stale without compinit counting
# every file at each start. bash's completions come along for the programs
# that bring only those.
() {
  emulate -L zsh
  zmodload -F zsh/files b:zf_mkdir
  local dump=${XDG_CACHE_HOME:-$HOME/.cache}/zsh/completions
  local made=${(j: :)${fpath:A}}
  zf_mkdir -p ${dump:h} 2> /dev/null
  autoload -Uz compinit bashcompinit
  if [[ -r $dump.from && "$(<$dump.from)" == "$made" && -r $dump ]]; then
    compinit -C -d $dump
  else
    compinit -i -d $dump
    print -r -- "$made" >| $dump.from
  fi
  [[ $dump.zwc -nt $dump ]] || zcompile -R -- $dump.zwc $dump
  bashcompinit
}
