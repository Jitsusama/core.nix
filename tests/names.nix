# How a repository's names are held to core's glossary, for core's own check and,
# through the flake's `lib.namesSayTheGlossary`, for a repository built on it,
# so every repository is held by one matcher to the glossary of the core it
# locks, and follows it as that lock moves.
{
  lib,
  python3,
  runCommandLocal,
}:
{
  # Every name to hold: each of the repository's checks' and options'.
  names,
  # Names whose word means something the glossary doesn't replace, each
  # with why.
  otherSenses ? { },
  # Names waiting on a rename their owner hasn't made, each with the owner
  # and the rename.
  waiting ? { },
}:
runCommandLocal "no-name-says-a-replaced-word"
  {
    names = builtins.toJSON names;
    kept = builtins.toJSON (lib.attrNames otherSenses ++ lib.attrNames waiting);
    passAsFile = [
      "names"
      "kept"
    ];
  }
  ''
    cp ${./glossary.py} glossary.py
    cp ${./names.py} names.py
    ${lib.getExe python3} names.py ${../docs/glossary.md} "$namesPath" "$keptPath"
    touch $out
  ''
