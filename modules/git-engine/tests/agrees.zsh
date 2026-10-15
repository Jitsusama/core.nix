# Puts repositories in the states a prompt meets and asks gitstatusd and
# git-engine about each through gitstatus's own plugin, counting without
# limits and stopping at one, and fails on any field they answer differently,
# git's file system monitor watching some of them. Where gitstatusd can't
# read a repository (reftable, a sparse index) or holds on to settings it
# read before, git-engine is held to what git itself says instead.
#
# zsh -i agrees.zsh PLUGIN-FOLDER GITSTATUSD ENGINE WORK, on a terminal, since
# the plugin wants job control.
emulate -L zsh
setopt err_return no_unset pipe_fail extended_glob

local plugin=$1 gitstatusd=$2 engine=$3 work=$4
typeset -g GITSTATUS_AUTO_INSTALL=0
export GIT_CONFIG_NOSYSTEM=1 GIT_AUTHOR_NAME=a GIT_AUTHOR_EMAIL=a@b \
  GIT_COMMITTER_NAME=a GIT_COMMITTER_EMAIL=a@b HOME=$work/home XDG_CACHE_HOME=$work/cache
mkdir -p $HOME
git config --global init.defaultBranch main
git config --global advice.detachedHead false
git config --global protocol.file.allow always

# nixpkgs' install script names gitstatusd outright, over whatever the
# caller asks for; a copy that leaves it to the caller.
mkdir -p $work/plugin
cp -r $plugin/. $work/plugin
chmod -R u+w $work/plugin
local -a install=("${(@f)$(<$work/plugin/install)}")
print -rl -- ${install:#GITSTATUS_DAEMON=*} > $work/plugin/install
# The plugin names its functions after its first argument; none, here.
source $work/plugin/gitstatus.plugin.zsh ''
# Each pair once counting everything, and once stopping at one of each, as
# the prompt has them.
typeset -g GITSTATUS_DAEMON=$gitstatusd
gitstatus_start -s -1 -u -1 -c -1 -d -1 theirs
gitstatus_start -s 1 -u 1 -c 1 -d 1 theirs_capped
typeset -g GITSTATUS_DAEMON=$engine
gitstatus_start -s -1 -u -1 -c -1 -d -1 ours
gitstatus_start -s 1 -u 1 -c 1 -d 1 ours_capped
# Each name has to be the daemon it's meant to be, or the comparison is of
# one with itself: only git-engine reads a reftable repository.
git init -q --ref-format=reftable $work/which
gitstatus_query -t -1 -d $work/which theirs
local theirs_said=$VCS_STATUS_RESULT
gitstatus_query -t -1 -d $work/which ours
if [[ $theirs_said != norepo-sync || $VCS_STATUS_RESULT != ok-sync ]]; then
  print -r -- "The daemons aren't the ones asked for: $theirs_said and $VCS_STATUS_RESULT."
  exit 1
fi

local -a fields=(
  VCS_STATUS_WORKDIR VCS_STATUS_COMMIT VCS_STATUS_LOCAL_BRANCH VCS_STATUS_REMOTE_BRANCH
  VCS_STATUS_REMOTE_NAME VCS_STATUS_REMOTE_URL VCS_STATUS_ACTION VCS_STATUS_INDEX_SIZE
  VCS_STATUS_NUM_STAGED VCS_STATUS_NUM_UNSTAGED VCS_STATUS_NUM_CONFLICTED
  VCS_STATUS_NUM_UNTRACKED VCS_STATUS_COMMITS_AHEAD VCS_STATUS_COMMITS_BEHIND
  VCS_STATUS_STASHES VCS_STATUS_TAG VCS_STATUS_NUM_UNSTAGED_DELETED VCS_STATUS_NUM_STAGED_NEW
  VCS_STATUS_NUM_STAGED_DELETED VCS_STATUS_PUSH_REMOTE_NAME VCS_STATUS_PUSH_REMOTE_URL
  VCS_STATUS_PUSH_COMMITS_AHEAD VCS_STATUS_PUSH_COMMITS_BEHIND VCS_STATUS_NUM_SKIP_WORKTREE
  VCS_STATUS_NUM_ASSUME_UNCHANGED VCS_STATUS_COMMIT_ENCODING VCS_STATUS_COMMIT_SUMMARY
)

# Every field one daemon gives for a folder, as name=value lines.
function answers() {
  local name=$1 dir=$2 field
  gitstatus_query -t -1 -d $dir $name
  print -r -- "result=${VCS_STATUS_RESULT}"
  [[ $VCS_STATUS_RESULT == ok-sync ]] || return 0
  for field in $fields; do
    print -r -- "${field#VCS_STATUS_}=${(P)field}"
  done
}

integer failed=0 checked=0
function agree() {
  local what=$1 dir=$2
  local theirs=$(answers theirs $dir) ours=$(answers ours $dir)
  (( ++checked ))
  if [[ $theirs != $ours ]]; then
    print -r -- "DIFFERS: $what"
    diff <(print -r -- $theirs) <(print -r -- $ours) || true
    (( ++failed ))
  fi
  theirs=$(answers theirs_capped $dir) ours=$(answers ours_capped $dir)
  (( ++checked ))
  if [[ $theirs != $ours ]]; then
    print -r -- "DIFFERS, counting to one: $what"
    diff <(print -r -- $theirs) <(print -r -- $ours) || true
    (( ++failed ))
  fi
}

# A field of git-engine's answer, for repositories gitstatusd can't read.
function ours() {
  local dir=$1 field=$2
  gitstatus_query -t -1 -d $dir ours
  print -r -- ${(P)${:-VCS_STATUS_$field}}
}

function expect() {
  local what=$1 actual=$2 wanted=$3
  (( ++checked ))
  if [[ $actual != $wanted ]]; then
    print -r -- "WRONG: $what: $actual, not $wanted"
    (( ++failed ))
  fi
}

# Holds git-engine's counts and branch to what git status says, where
# gitstatusd can't read the repository at all, or holds on to settings it
# read before. git is kept from refreshing the index, which would change
# what the next question sees.
function like_git() {
  local what=$1 dir=$2 line
  integer staged=0 unstaged=0 untracked=0 conflicted=0
  for line in "${(@f)$(git -C $dir --no-optional-locks status --porcelain=v2)}"; do
    case $line in
      ([12]\ *) [[ ${line[3]} != . ]] && (( ++staged )); [[ ${line[4]} != . ]] && (( ++unstaged )) ;;
      (u\ *) (( ++conflicted )) ;;
      (\?\ *) (( ++untracked )) ;;
    esac
  done
  gitstatus_query -t -1 -d $dir ours
  expect "$what: branch" "$VCS_STATUS_LOCAL_BRANCH" "$(git -C $dir branch --show-current)"
  expect "$what: commit" "$VCS_STATUS_COMMIT" "$(git -C $dir rev-parse HEAD)"
  expect "$what: staged" "$VCS_STATUS_NUM_STAGED" $staged
  expect "$what: unstaged" "$VCS_STATUS_NUM_UNSTAGED" $unstaged
  expect "$what: untracked" "$VCS_STATUS_NUM_UNTRACKED" $untracked
  expect "$what: conflicted" "$VCS_STATUS_NUM_CONFLICTED" $conflicted
}

# Has git's file system monitor watch a repository, and git-engine ask it.
function watch() {
  git -C $1 config core.fsmonitor true
  if ! git -C $1 fsmonitor--daemon start; then
    print -r -- "WRONG: git's file system monitor wouldn't start for ${1:t}"
    (( ++failed ))
  fi
}

function fresh() {
  local dir=$work/$1
  rm -rf $dir
  git init -q $dir
  print -r -- $dir
}

local r

# Files only a look at their contents can judge, since their stat moved,
# made first and asked about last: a content check is remembered only for a
# file left alone for two seconds, and these must be remembered to show that
# what's remembered goes when how the file reads changes.
for r in $(fresh moved) $(fresh moved-watched); do
  git -C $r config core.autocrlf true
  printf 'y\r\n' > $r/lines; git -C $r add lines 2> /dev/null
  git -C $r commit -qm lines
  touch -d 2020-01-01 $r/lines
done

r=$(fresh unborn)
agree "a repository with no commits" $r
print one > $r/a
agree "an untracked file before the first commit" $r
git -C $r add a
agree "a file staged for the first commit" $r

r=$(fresh basic)
print one > $r/a; print two > $r/b; mkdir -p $r/d/e; print three > $r/d/e/f
git -C $r add -A; git -C $r commit -qm "first one" -m "a body, which the summary leaves out"
agree "a clean commit" $r
print more >> $r/a
agree "a changed file" $r
git -C $r add a
agree "a staged change" $r
print again >> $r/a
agree "a staged change changed again" $r
git -C $r rm -q b
agree "a staged deletion" $r
rm $r/d/e/f
agree "a file deleted in the worktree" $r
git -C $r checkout -q -- .
git -C $r reset -q --hard
agree "back to clean" $r
print new > $r/new; mkdir $r/newdir; print x > $r/newdir/x; mkdir $r/empty
agree "untracked files, a folder of them and an empty folder" $r
rm -rf $r/new $r/newdir $r/empty

# Ignore files at every level, negation, folders-only and anchored ones.
print -l '*.log' '/build/' 'cache/' '!keep.log' > $r/.gitignore
mkdir -p $r/build $r/d/cache $r/d/build
print x > $r/a.log; print x > $r/keep.log; print x > $r/build/out; print x > $r/d/cache/c
print x > $r/d/build/out
print -l 'local' > $r/d/.gitignore; print x > $r/d/local; print x > $r/d/notlocal
print 'excluded' > $r/.git/info/exclude; print x > $r/excluded
agree "ignore rules of every kind" $r
git -C $r add .gitignore d/.gitignore; git -C $r commit -qm ignores
git -C $r clean -qfdx

# Stat changes that aren't content changes, and ones that are.
touch $r/a
agree "a file touched but unchanged" $r
print one > $r/a.tmp; mv $r/a.tmp $r/a
agree "a file rewritten with the same contents" $r
chmod +x $r/a
agree "a file made executable" $r
chmod -x $r/a
ln -s a $r/link; git -C $r add link; git -C $r commit -qm link
ln -sf b $r/link
agree "a symlink pointed elsewhere" $r
git -C $r checkout -q -- link
rm $r/a; mkdir $r/a; print x > $r/a/inside
agree "a file replaced by a folder" $r
rm -rf $r/a; git -C $r checkout -q -- a

# Intent to add, and the two ways of telling git to look away.
print later > $r/later; git -C $r add -N later
agree "a file added with intent to add" $r
git -C $r rm -q --cached later; rm $r/later
git -C $r update-index --assume-unchanged b
git -C $r update-index --skip-worktree d/e/f
agree "assume-unchanged and skip-worktree" $r
git -C $r update-index --no-assume-unchanged b
git -C $r update-index --no-skip-worktree d/e/f

# Index version 4, with its compressed names.
git -C $r update-index --index-version 4
print more >> $r/d/e/f
agree "index version 4" $r
git -C $r checkout -q -- .
git -C $r update-index --index-version 2

# Packed objects, deltas and a commit-graph.
local i
for i in {1..30}; do print "line $i" >> $r/a; git -C $r commit -qam "change $i"; done
git -C $r gc -q --aggressive
git -C $r commit-graph write --reachable
agree "packed, with a commit-graph" $r
for i in {31..33}; do print "line $i" >> $r/a; git -C $r commit -qam "change $i"; done
git -C $r commit-graph write --reachable --split
agree "a split commit-graph" $r

# Tags, plain and annotated, and stashes.
git -C $r tag light
agree "a lightweight tag" $r
git -C $r tag -d light; git -C $r tag -am note annotated
agree "an annotated tag" $r
print stash >> $r/a; git -C $r stash -q
print stash2 >> $r/b; git -C $r stash -q
agree "two stashes" $r
git -C $r pack-refs --all
agree "packed refs" $r

# An upstream, ahead and behind, through a remote and a merge.
local remote=$work/remote.git
rm -rf $remote; git init -q --bare $remote
git -C $r remote add origin $remote
git -C $r push -q -u origin main
agree "level with its upstream" $r
print ahead >> $r/b; git -C $r commit -qam ahead
agree "one ahead" $r
local other=$work/other
rm -rf $other; git clone -q $remote $other
for i in 1 2; do print "theirs $i" >> $other/d/e/f; git -C $other commit -qam "theirs $i"; done
git -C $other push -q
git -C $r fetch -q
agree "ahead and behind" $r
git -C $r merge -q --no-edit origin/main
agree "ahead after a merge" $r
git -C $r config branch.main.pushRemote origin
agree "with a push remote" $r
git -C $r push -q

# A conflict, a rebase stopped on one, and a detached HEAD.
git -C $r checkout -qb side
print side > $r/b; git -C $r commit -qam side
git -C $r checkout -q main
print main > $r/b; git -C $r commit -qam main
git -C $r merge -q side 2>/dev/null || true
agree "a merge with a conflict" $r
git -C $r merge --abort
git -C $r rebase -q side 2>/dev/null || true
agree "a rebase stopped on a conflict" $r
git -C $r rebase --abort
git -C $r checkout -q --detach HEAD~1
agree "a detached HEAD" $r
git -C $r checkout -q main

# A linked worktree, with its own HEAD and index.
git -C $r worktree add -q $work/linked side
print linked >> $work/linked/b
agree "a linked worktree" $work/linked
agree "a folder inside a linked worktree" $work/linked/d

# A submodule.
local sub=$(fresh sub)
print s > $sub/s; git -C $sub add s; git -C $sub commit -qm s
git -C $r submodule -q add $sub sub 2>/dev/null
git -C $r commit -qm submodule
agree "a submodule" $r
print dirty >> $r/sub/s
agree "a submodule changed inside" $r

# Line endings: autocrlf input with a CRLF file, and text attributes.
r=$(fresh crlf)
git -C $r config core.autocrlf input
printf 'one\r\ntwo\r\n' > $r/dos; printf 'bin\0\r\n' > $r/binary
print '*.txt text' > $r/.gitattributes; printf 'text\r\n' > $r/t.txt
git -C $r add -A 2>/dev/null; git -C $r commit -qm crlf 2>/dev/null
touch $r/dos $r/binary $r/t.txt
agree "CRLF files with autocrlf input" $r

# A sparse checkout, held to git: libgit2 takes a file left out of the
# checkout for deleted, and can't read a sparse index at all.
r=$(fresh sparse)
mkdir -p $r/in/deep $r/out/deep
print i > $r/in/deep/i; print o > $r/out/deep/o; print top > $r/top
git -C $r add -A; git -C $r commit -qm sparse
git -C $r sparse-checkout set --cone in
like_git "a sparse checkout" $r
print more >> $r/in/deep/i
like_git "a change in a sparse checkout" $r
git -C $r checkout -q -- .
git -C $r sparse-checkout set --cone --sparse-index in
like_git "a sparse index" $r
print more >> $r/in/deep/i; print new > $r/in/new; print new > $r/top2
like_git "changes beside a sparse index" $r
git -C $r add in/new
like_git "a file staged beside a sparse index" $r

# reftable, which gitstatusd can't read: held to git instead.
r=$work/reftable
rm -rf $r; git init -q --ref-format=reftable $r
print one > $r/a; git -C $r add a; git -C $r commit -qm one
git -C $r checkout -qb work
print two >> $r/a; git -C $r commit -qam two
git -C $r branch -q --set-upstream-to main
print three >> $r/a
git -C $r stash -q
print four >> $r/a
expect "reftable's branch" "$(ours $r LOCAL_BRANCH)" work
expect "reftable's commit" "$(ours $r COMMIT)" "$(git -C $r rev-parse HEAD)"
expect "reftable's ahead" "$(ours $r COMMITS_AHEAD)" 1
expect "reftable's unstaged" "$(ours $r NUM_UNSTAGED)" 1
expect "reftable's stashes" "$(ours $r STASHES)" 1
git -C $r stash drop -q
expect "reftable's stashes once dropped" "$(ours $r STASHES)" 0
git -C $r pack-refs
expect "reftable's branch, compacted" "$(ours $r LOCAL_BRANCH)" work
git -C $r worktree add -q $work/reftable-linked main 2>/dev/null
expect "reftable's linked worktree" "$(ours $work/reftable-linked LOCAL_BRANCH)" main

# Watched by git's file system monitor, which git-engine asks what changed
# and gitstatusd doesn't: each change has to show up all the same.
r=$(fresh watched)
mkdir -p $r/d/e $r/quiet
print one > $r/a; print two > $r/d/e/f; print q > $r/quiet/q; printf 'x\r\n' > $r/crlf
print '*.log' > $r/.gitignore
git -C $r add -A; git -C $r commit -qm watched
watch $r
agree "watched and clean" $r
agree "watched and asked again" $r
print more >> $r/a
agree "a watched file changed" $r
agree "a watched file changed, asked again" $r
git -C $r checkout -q -- a
agree "a watched file put back" $r
print new > $r/d/new
agree "a new file in a watched folder" $r
mkdir -p $r/fresh/deeper; print x > $r/fresh/deeper/x
agree "a new folder of new files" $r
print x > $r/d/e/skip.log
agree "a new ignored file" $r
print 'new' >> $r/.gitignore
agree "an ignore file changed to cover a new file" $r
rm $r/d/e/f
agree "a watched file deleted" $r
git -C $r checkout -q -- d/e/f
rm -rf $r/fresh $r/d/new $r/d/e/skip.log; git -C $r checkout -q -- .gitignore
agree "all put back" $r
git -C $r config core.autocrlf true
agree "line endings asked for differently" $r
print 'crlf -text' > $r/.gitattributes
agree "an attributes file changing how a file reads" $r
rm $r/.gitattributes; git -C $r config --unset core.autocrlf
agree "line endings as they were" $r
print s >> $r/quiet/q; git -C $r add quiet/q
agree "a change staged while watched" $r
git -C $r fsmonitor--daemon stop
agree "the monitor stopped" $r

# The monitor losing track: a new one can't name what changed while none
# was watching, so what was taken on the old one's word has to be looked at
# again. With untracked files not asked about, the question the new one
# answers first looks at no folder, so only forgetting finds the new file.
r=$(fresh restarted)
mkdir -p $r/a $r/z; print a > $r/a/a; print z > $r/z/z
git -C $r add -A; git -C $r commit -qm two
watch $r
agree "watched, before the monitor restarts" $r
agree "watched, asked again before it restarts" $r
git -C $r config status.showUntrackedFiles no
git -C $r fsmonitor--daemon stop
print new > $r/z/new
watch $r
like_git "asked while not counting untracked files" $r
git -C $r config --unset status.showUntrackedFiles
like_git "a file made while no monitor watched" $r
git -C $r fsmonitor--daemon stop

# The moved files: clean while autocrlf turns their CRLF into the blob's LF,
# changed once it doesn't, and clean again once an attributes file asks for
# the same. gitstatusd keeps the autocrlf it read first, so git judges these.
for r in $work/moved $work/moved-watched; do
  [[ $r == *-watched ]] && watch $r
  like_git "${r:t}: a moved file read through autocrlf" $r
  git -C $r config --unset core.autocrlf
  like_git "${r:t}: the moved file read without autocrlf" $r
  print 'lines text' > $r/.gitattributes
  like_git "${r:t}: the moved file read as text by its attributes" $r
  rm $r/.gitattributes
  like_git "${r:t}: the moved file without its attributes" $r
  [[ $r == *-watched ]] && git -C $r fsmonitor--daemon stop
done

# Outside any repository.
mkdir -p $work/none
gitstatus_query -t -1 -d $work/none ours
expect "a folder in no repository" $VCS_STATUS_RESULT norepo-sync

gitstatus_stop theirs
gitstatus_stop ours
print -r -- "$checked checked, $failed failed"
(( failed == 0 ))
