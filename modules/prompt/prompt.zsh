# The prompt. Before the glyph, and never moving: where Joel is, told from
# the nearest root, another machine's name over SSH, and a rebase, merge or
# pick under way, drawn as its steps where it has them. The glyph is where
# his keys go: the accent, red after a failure, # in the warning's gold as
# root. After it, receding: a development shell and background jobs as bare
# glyphs, how long a long command ran, and the branch with git's notation
# for its changes and its upstream.
#
# It's laid out once per prompt and again on a resize, never per keystroke.
# When the window is too narrow the right side gives way a step at a time,
# the least needed first, and what gives way goes without a trace; a name
# cut short keeps the more mark where it was cut. zsh clears the right side
# whole once typing reaches it. Once a command is entered
# its prompt steps back to the folder and a muted glyph, so scrollback shows
# the commands, and a blank line separates each command from the last.
#
# Nothing here starts a process once the shell is up. git's state comes from
# git-engine, which answers as gitstatusd does, through gitstatus's plugin,
# started once and asked without waiting. Where it isn't (a host the ssh
# kitten carried this file to) the branch is read from HEAD.
#
# The header written with this file sets _prompt_hue (the theme's roles),
# _prompt_glyph and _prompt_console (its marks and their console stand-ins),
# _prompt_roots, _prompt_steps, _prompt_long and _prompt_gitstatus.

# Once per shell: a machine of Joel's that kitty carried this to has it
# from its own setup already.
(( $+functions[_prompt_precmd] )) && return

# A dumb terminal, as an editor's shell is, gets a plain $ and nothing to
# redraw.
if [[ $TERM == dumb ]]; then
  unsetopt zle prompt_cr prompt_subst
  PROMPT='$ ' RPROMPT=''
  return
fi

zmodload zsh/datetime zsh/parameter zsh/system
autoload -Uz add-zle-hook-widget
setopt prompt_subst prompt_percent

[[ $TERM == linux ]] && _prompt_glyph=("${(@kv)_prompt_console}")
ZLE_RPROMPT_INDENT=0
# What's typed is the star, so it wears the strong text, in scrollback too.
zle_highlight=(${zle_highlight:#default:*} "default:fg=${_prompt_hue[strong]}")

typeset -g _prompt_left _prompt_right _prompt_past _prompt_typed
typeset -g _prompt_l _prompt_r _prompt_short _prompt_took _prompt_ran
typeset -gi _prompt_level _prompt_status _prompt_drawn _prompt_asked
typeset -gF _prompt_started
typeset -gA _prompt_state

PROMPT='${_prompt_left}'
RPROMPT='${_prompt_right}'

# Appends one coloured piece to the line being built, keeping its plain
# text beside it so its width can be counted.
_prompt_put() {
  REPLY+="%F{${_prompt_hue[$1]}}${2//\%/%%}%f"
  _prompt_plain+=$2
}

_prompt_gap() {
  [[ -n $_prompt_plain ]] || return 0
  REPLY+="  "
  _prompt_plain+="  "
}

_prompt_on() {
  [[ $1 == pathshort && -n $_prompt_short ]] && return 0
  local at=${_prompt_steps[(I)$1]}
  (( at && at <= _prompt_level ))
}

# A name cut in its middle to keep both ends.
_prompt_middle() {
  local name=$1 keep=$2
  if (( ${#name} <= keep )); then
    REPLY=$name
    return
  fi
  local head=$(( (keep - 1) / 2 )) tail=$(( keep - 1 - (keep - 1) / 2 ))
  REPLY="${name[1,head]}${_prompt_glyph[more]}${name[-tail,-1]}"
}

# Reads the first line of a file without starting anything.
_prompt_line() {
  REPLY=""
  [[ -r $1 ]] && IFS= read -r REPLY < $1
  [[ -n $REPLY ]]
}

# What git is in the middle of, from the files it keeps while it's there.
_prompt_operation() {
  local dir=$1 at="" of=""
  _prompt_state[op]="" _prompt_state[at]="" _prompt_state[of]=""
  [[ -n $dir ]] || return
  if [[ -d $dir/rebase-merge ]]; then
    _prompt_state[op]=rebase
    _prompt_line $dir/rebase-merge/msgnum && at=$REPLY
    _prompt_line $dir/rebase-merge/end && of=$REPLY
  elif [[ -d $dir/rebase-apply ]]; then
    _prompt_state[op]=rebase
    [[ -e $dir/rebase-apply/applying ]] && _prompt_state[op]=am
    _prompt_line $dir/rebase-apply/next && at=$REPLY
    _prompt_line $dir/rebase-apply/last && of=$REPLY
  elif [[ -e $dir/MERGE_HEAD ]]; then
    _prompt_state[op]=merge
  elif [[ -e $dir/CHERRY_PICK_HEAD ]]; then
    _prompt_state[op]=pick
  elif [[ -e $dir/REVERT_HEAD ]]; then
    _prompt_state[op]=revert
  elif [[ -e $dir/BISECT_LOG ]]; then
    _prompt_state[op]=bisect
  fi
  if [[ $at == <1-> && $of == <1-> ]]; then
    _prompt_state[at]=$at _prompt_state[of]=$of
  fi
}

# Where Joel is, and the repository around it: the nearest root marker
# names the root, and the nearest .git names the repository.
_prompt_where() {
  local dir=$PWD root="" git="" marker
  while :; do
    if [[ -z $root ]]; then
      for marker in $_prompt_roots; do
        [[ -e $dir/$marker ]] && { root=$dir; break }
      done
    fi
    [[ -z $git && -e $dir/.git ]] && git=$dir/.git
    [[ -n $root && -n $git || $dir == / ]] && break
    dir=${dir:h}
  done
  _prompt_state[root]=$root
  # A worktree's .git is a file naming where its repository keeps it.
  if [[ -f $git ]] && _prompt_line $git && [[ $REPLY == gitdir:\ * ]]; then
    local named=${REPLY#gitdir: }
    [[ $named == /* ]] || named=${git:h}/$named
    git=$named
  fi
  _prompt_state[gitdir]=$git
}

# The byte at a place in the block being read, as a number.
_prompt_byte() {
  local char=${block[$1]}
  (( byte = #char ))
}

# One of git's varints from the block being read, moving past it.
_prompt_varint() {
  _prompt_byte $at
  (( value = byte & 127, at++ ))
  while (( byte & 128 )); do
    _prompt_byte $at
    (( value = ((value + 1) << 7) | (byte & 127), at++ ))
  done
}

# What HEAD names in a repository that keeps its refs in reftable, as
# World does: HEAD's file then says only refs/heads/.invalid. HEAD sorts
# before every refs/ name, so each table's first block holds it if the
# table does, and the newest table holding it says what it is.
_prompt_reftable() {
  emulate -L zsh -o no_multibyte -o extended_glob
  local dir=$1/reftable table block key hex
  local -i fd at end kind length value hash prefix byte size i
  local -a tables
  [[ -r $dir/tables.list ]] || return
  tables=(${(f)"$(<$dir/tables.list)"})
  for table in ${(Oa)tables}; do
    [[ -r $dir/$table ]] || continue
    # The header, then the first block, whose length the header gives.
    exec {fd}<$dir/$table
    block=""
    sysread -s 24 -i $fd block
    [[ ${block[1,4]} == REFT ]] || { exec {fd}<&-; continue }
    size=0
    for i in 6 7 8; do _prompt_byte $i; (( size = size << 8 | byte )); done
    (( size < 24 || size > 65536 )) && size=4096
    local rest=""
    sysread -s $(( size - 24 )) -i $fd rest
    exec {fd}<&-
    block+=$rest
    [[ ${block[25]} == r ]] || continue
    _prompt_byte 5
    (( hash = byte == 2 ? 32 : 20 ))
    end=0
    for i in 26 27 28; do _prompt_byte $i; (( end = end << 8 | byte )); done
    (( end > ${#block} )) && end=${#block}
    _prompt_byte $(( end - 1 )); (( i = byte << 8 ))
    _prompt_byte $end; (( end -= 2 + 3 * (i | byte) ))
    at=29 key=""
    while (( at <= end )); do
      _prompt_varint; prefix=value
      _prompt_varint; (( length = value >> 3, kind = value & 7 ))
      key=${key[1,prefix]}${block[at,at + length - 1]}
      (( at += length ))
      _prompt_varint
      if [[ $key == HEAD ]]; then
        REPLY=""
        case $kind in
          (1|2)
            hex=""
            for i in {0..3}; do
              _prompt_byte $(( at + i ))
              hex+=${(l:2::0:)$(( [##16] byte ))}
            done
            REPLY=${(L)hex[1,7]}
            ;;
          (3) _prompt_varint; REPLY="ref: ${block[at,at + value - 1]}" ;;
        esac
        [[ -n $REPLY ]] && return 0
        break
      fi
      [[ $key > HEAD ]] && break
      case $kind in
        (1) (( at += hash )) ;;
        (2) (( at += 2 * hash )) ;;
        (3) _prompt_varint; (( at += value )) ;;
      esac
    done
  done
  return 1
}

# The branch as HEAD names it, for where the daemon isn't, or can't read.
_prompt_head() {
  _prompt_state[branch]=""
  local dir=${_prompt_state[gitdir]}
  [[ -n $dir ]] && _prompt_line $dir/HEAD || return
  [[ $REPLY == 'ref: refs/heads/.invalid' ]] && { _prompt_reftable $dir || return }
  if [[ $REPLY == ref:\ refs/heads/* ]]; then
    _prompt_state[branch]=${REPLY#ref: refs/heads/}
  else
    _prompt_state[branch]=${REPLY[1,7]}
  fi
}

# What the daemon answered, when it answered for this folder.
_prompt_git() {
  _prompt_clean
  [[ $VCS_STATUS_RESULT == ok-* ]] || return
  if [[ -n $VCS_STATUS_LOCAL_BRANCH ]]; then
    _prompt_state[branch]=$VCS_STATUS_LOCAL_BRANCH
  else
    _prompt_state[branch]=${VCS_STATUS_COMMIT[1,7]}
  fi
  (( VCS_STATUS_HAS_UNSTAGED )) && _prompt_state[unstaged]=1
  (( VCS_STATUS_HAS_STAGED )) && _prompt_state[staged]=1
  (( VCS_STATUS_HAS_UNTRACKED )) && _prompt_state[untracked]=1
  (( VCS_STATUS_COMMITS_AHEAD > 0 )) && _prompt_state[ahead]=$VCS_STATUS_COMMITS_AHEAD
  (( VCS_STATUS_COMMITS_BEHIND > 0 )) && _prompt_state[behind]=$VCS_STATUS_COMMITS_BEHIND
  (( VCS_STATUS_HAS_CONFLICTED )) && _prompt_state[conflicted]=1
}

# A tree with nothing to tell: clean, in step, no conflict.
_prompt_clean() {
  _prompt_state[unstaged]=0 _prompt_state[staged]=0 _prompt_state[untracked]=0
  _prompt_state[ahead]=0 _prompt_state[behind]=0 _prompt_state[conflicted]=0
}

# The tree's changes and the upstream after the branch, in the notation
# git's own short status and the usual prompts use: * not staged, + staged,
# ? untracked, together, then ↑N ahead and ↓N behind, together. A tree
# clean and in step adds nothing.
_prompt_notation() {
  local changes="" upstream=""
  (( ${_prompt_state[unstaged]} )) && changes+='*'
  (( ${_prompt_state[staged]} )) && changes+='+'
  (( ${_prompt_state[untracked]} )) && changes+='?'
  (( ${_prompt_state[ahead]} )) && upstream+="↑${_prompt_state[ahead]}"
  (( ${_prompt_state[behind]} )) && upstream+="↓${_prompt_state[behind]}"
  local group
  for group in $changes $upstream; do
    REPLY+=" " _prompt_plain+=" "
    _prompt_put text $group
  done
}

# Where Joel is: the root's name in text, the folders between muted, the one
# he's in strongest. Outside a root the path is told from home or from /.
_prompt_path() {
  local root=${_prompt_state[root]} lead="" rest first=muted
  local -a parts
  if [[ -n $root ]]; then
    parts=(${root:t} ${(s:/:)${PWD#$root}})
    first=text
  elif [[ $PWD == $HOME || $PWD == $HOME/* ]]; then
    parts=('~' ${(s:/:)${PWD#$HOME}})
    first=text
  else
    lead=/
    parts=(${(s:/:)PWD})
  fi
  if (( ${#parts} == 0 )); then
    _prompt_put strong /
  elif (( ${#parts} == 1 )); then
    _prompt_put strong $lead${parts[1]}
  else
    local between=${(j:/:)parts[2,-2]}
    _prompt_on pathshort && (( ${#parts} > 2 )) && between=${_prompt_glyph[more]}
    _prompt_put $first $lead${parts[1]}
    _prompt_put muted /${between:+$between/}
    _prompt_put strong ${parts[-1]}
  fi
}

# A rebase's steps in the table's marks: those done, this one, those to
# come. Past eight the numbers say it in less room.
_prompt_progress() {
  local at=${_prompt_state[at]} of=${_prompt_state[of]} hue=$1 i
  REPLY+=" " _prompt_plain+=" "
  if (( of > 8 )); then
    _prompt_put text $at/$of
    return
  fi
  for (( i = 1; i <= of; i++ )); do
    if (( i < at )); then _prompt_put text ${_prompt_glyph[complete]}
    elif (( i == at )); then _prompt_put $hue ${_prompt_glyph[active]}
    else _prompt_put muted ${_prompt_glyph[pending]}
    fi
  done
}

_prompt_build_left() {
  local REPLY="" _prompt_plain=""
  _prompt_path
  [[ -n ${_prompt_state[host]} ]] && {
    REPLY+=" " _prompt_plain+=" "
    _prompt_put machine ${_prompt_glyph[elsewhere]}
    _prompt_put text ${_prompt_state[host]}
  }
  local op=${_prompt_state[op]}
  if [[ -n $op ]]; then
    # Red while a conflict waits on Joel, gold while it simply runs.
    local hue=warning
    (( ${_prompt_state[conflicted]} )) && hue=alert
    REPLY+="  " _prompt_plain+="  "
    if _prompt_on opshort; then
      _prompt_put $hue ${_prompt_glyph[active]}
    elif [[ -n ${_prompt_state[of]} ]]; then
      _prompt_put text $op
      _prompt_progress $hue
    else
      _prompt_put $hue ${_prompt_glyph[active]}
      REPLY+=" " _prompt_plain+=" "
      _prompt_put text $op
    fi
  fi
  # As root the keys still go to the glyph, but they reach everything.
  local mark=${_prompt_glyph[prompt]} hue=accent
  (( EUID == 0 )) && mark=${_prompt_glyph[root]} hue=warning
  (( _prompt_status )) && hue=alert
  REPLY+=" " _prompt_plain+=" "
  _prompt_put $hue $mark
  REPLY+=" " _prompt_plain+=" "
  _prompt_l=$REPLY
  _prompt_lw=${(m)#_prompt_plain}
}

_prompt_build_right() {
  local REPLY="" _prompt_plain=""
  _prompt_r="" _prompt_rw=0
  _prompt_on right && return
  if { [[ -n ${_prompt_state[shell]} ]] || (( ${_prompt_state[jobs]} )) } && ! _prompt_on shell; then
    [[ -n ${_prompt_state[shell]} ]] && _prompt_put program ${_prompt_glyph[shell]}
    if (( ${_prompt_state[jobs]} )); then
      [[ -n ${_prompt_state[shell]} ]] && REPLY+=" " _prompt_plain+=" "
      _prompt_put voice ${_prompt_glyph[running]}
    fi
  fi
  if [[ -n $_prompt_took ]] && ! _prompt_on time; then
    _prompt_gap
    _prompt_put muted $_prompt_took
  fi
  local branch=${_prompt_state[branch]}
  if [[ -n $branch ]]; then
    _prompt_gap
    _prompt_put muted ${_prompt_glyph[branch]}
    if ! _prompt_on branchglyph; then
      if _prompt_on branchshort; then
        local whole=$REPLY
        _prompt_middle $branch 14
        branch=$REPLY REPLY=$whole
      fi
      REPLY+=" " _prompt_plain+=" "
      _prompt_put muted $branch
    fi
    _prompt_notation
  fi
  _prompt_r=$REPLY
  _prompt_rw=${(m)#_prompt_plain}
}

# Lays the prompt out for the window as it is: the left keeps to a third of
# it, and room is left to start typing.
_prompt_lay() {
  emulate -L zsh -o extended_glob
  local -i _prompt_lw _prompt_rw cols=${COLUMNS:-80}
  _prompt_short=""
  _prompt_level=0
  _prompt_build_left
  (( _prompt_lw * 3 > cols )) && _prompt_short=1
  for (( _prompt_level = 0; _prompt_level <= ${#_prompt_steps}; _prompt_level++ )); do
    _prompt_build_left
    _prompt_build_right
    (( _prompt_lw + 8 + (_prompt_rw ? _prompt_rw + 2 : 0) <= cols )) && break
  done
  _prompt_left=$_prompt_l
  _prompt_right=$_prompt_r
}

# The prompt a command leaves in scrollback: the folder it ran in and a
# muted glyph, red when the command before it failed, so the red sits
# under the output that failed.
_prompt_stepped_back() {
  local REPLY="" _prompt_plain="" mark=${_prompt_glyph[prompt]} hue=muted
  (( EUID == 0 )) && mark=${_prompt_glyph[root]}
  (( _prompt_status )) && hue=alert
  local -a parts=(${(s:/:)PWD})
  _prompt_put muted ${${parts[-1]}:-/}
  [[ -n ${_prompt_state[host]} ]] && {
    REPLY+=" " _prompt_plain+=" "
    _prompt_put muted ${_prompt_glyph[elsewhere]}${_prompt_state[host]}
  }
  REPLY+=" " _prompt_plain+=" "
  _prompt_put $hue $mark
  _prompt_past="$REPLY "
}

_prompt_git_arrived() {
  emulate -L zsh -o extended_glob
  (( _prompt_asked )) || return
  _prompt_git
  _prompt_lay
  zle && zle .reset-prompt
}

_prompt_precmd() {
  local -i ret=$?
  emulate -L zsh -o extended_glob
  # A line cancelled at the prompt ran nothing, so it leaves the last
  # command's outcome alone.
  if [[ -n $_prompt_ran ]]; then
    _prompt_status=ret
    _prompt_took=""
    if (( _prompt_started )); then
      local -F took=$(( EPOCHREALTIME - _prompt_started ))
      local -i whole=took
      if (( took >= _prompt_long )); then
        if (( took < 60 )); then printf -v _prompt_took '%.1fs' took
        elif (( took < 3600 )); then
          printf -v _prompt_took '%dm %02ds' $(( whole / 60 )) $(( whole % 60 ))
        else printf -v _prompt_took '%dh %02dm' $(( whole / 3600 )) $(( whole % 3600 / 60 ))
        fi
      fi
    fi
  fi
  _prompt_started=0
  # A line of air between a command's output and the next prompt, except
  # after clearing the screen.
  (( _prompt_drawn )) && [[ -n $_prompt_ran && ${_prompt_ran%% *} != (clear|reset) ]] && print
  _prompt_drawn=1
  _prompt_ran=""

  _prompt_state[host]=""
  [[ -n $SSH_CONNECTION ]] && _prompt_state[host]=${HOST%%.*}
  _prompt_state[shell]=$IN_NIX_SHELL
  _prompt_state[jobs]=${#jobstates}
  _prompt_where
  _prompt_operation ${_prompt_state[gitdir]}
  _prompt_head
  _prompt_clean
  _prompt_asked=0
  # The daemon was started without waiting for it; the first prompt takes its
  # hello, which the rest of the shell's start has usually given it time for.
  (( $+functions[gitstatus_start_prompt] )) && gitstatus_start_prompt $_prompt_daemon prompt
  if [[ -n ${_prompt_state[gitdir]} ]] && (( $+functions[gitstatus_query_prompt] )); then
    _prompt_asked=1
    # A moment's wait answers most folders in time for the first draw, so
    # the marks don't arrive a beat after the prompt.
    gitstatus_query_prompt -t 0.02 -c _prompt_git_arrived prompt || _prompt_asked=0
    [[ $VCS_STATUS_RESULT == ok-sync ]] && _prompt_git
  fi
  _prompt_lay
  # Set again each time, in case another machine's own prompt took them.
  PROMPT='${_prompt_left}'
  RPROMPT='${_prompt_right}'
}

_prompt_preexec() {
  _prompt_ran=$1
  _prompt_started=$EPOCHREALTIME
}

_prompt_line_finish() {
  _prompt_asked=0
  _prompt_stepped_back
  _prompt_left=$_prompt_past
  _prompt_right=""
  zle .reset-prompt
}

TRAPWINCH() {
  _prompt_lay
  zle && zle .reset-prompt
}

precmd_functions+=(_prompt_precmd)
preexec_functions+=(_prompt_preexec)
add-zle-hook-widget line-finish _prompt_line_finish

if [[ -r $_prompt_gitstatus ]]; then
  zmodload zsh/zselect
  # As many threads as gitstatus would choose, counted from the processors
  # the kernel lists rather than asked of getconf.
  [[ -n $GITSTATUS_NUM_THREADS ]] || () {
    local -i cpus=0
    local range
    if _prompt_line /sys/devices/system/cpu/online; then
      for range in ${(s:,:)REPLY}; do
        if [[ $range == *-* ]]; then
          cpus+=$(( ${range#*-} - ${range%-*} + 1 ))
        else
          cpus+=1
        fi
      done
    fi
    (( cpus )) || cpus=8
    typeset -g GITSTATUS_NUM_THREADS=$(( cpus > 16 ? 32 : 2 * cpus ))
  }
  source $_prompt_gitstatus _prompt
  typeset -ga _prompt_daemon=(-s 1 -u 1 -c 1 -d 1)
  gitstatus_start_prompt -a $_prompt_daemon prompt
fi
