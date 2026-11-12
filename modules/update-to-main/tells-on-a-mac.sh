# shellcheck shell=bash
# What a Mac's follower runs to tell its person the Mac can't update
# (src/tell.rs), as a notice in their session. Why it can't comes as the
# one argument; the notice says it in a person's words instead, and where
# the follower's log keeps the why.
title="This Mac couldn't update"
body="It tries again once there's something newer. Why is in Library/Logs/update-to-main-follows.log."
exec /usr/bin/osascript \
  -e 'on run argv' \
  -e 'display notification (item 2 of argv) with title (item 1 of argv)' \
  -e 'end run' \
  "$title" "$body"
