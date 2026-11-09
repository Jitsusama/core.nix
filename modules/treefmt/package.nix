# treefmt, patched so that a file no formatter reads fails a run at once
# under on-unmatched = fatal. Released treefmt stops reading git's list of
# files on that error but waits for git to finish writing it, which never
# happens once the list outgrows a pipe: the formatting check then runs
# until it's killed rather than naming the file. Upstream's main fixed it in
# rewriting the walker (numtide/treefmt f4b79f9), after 2.6.0; this goes
# once a release carries that.
{ treefmt }:
treefmt.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [ ./git-walker-stops.patch ];
})
