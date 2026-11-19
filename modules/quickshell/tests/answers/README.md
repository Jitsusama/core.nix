# Answers

The cases of the launcher's answers, one file a place: what's typed into it
and what has to come back, or that nothing should. `../answers.qml` imports
each file and runs its `run` in turn, in the order it lists them, so two
places' cases never share a file. A new place adds its file to both of that
root's lists.

`common.js` holds what more than one place's cases use: quoting what's typed,
reading a file the check copies beside the root, and the checker that counts
each case asked.
