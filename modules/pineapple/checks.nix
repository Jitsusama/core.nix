# The checks of pineapple, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: every saying in sayings.json, the real
  # messages of 13 November and 20 October among them, is judged the safe
  # word, a resume, an ask to stop or a near miss (no tag, a quote, talked
  # about, a joke, a bot) as the table says, and reaches what its words
  # and the place it was said in name, or everything, said so.
  pineapple-judges-each-saying = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pineapple, its tests included.
  pineapple-passes-clippy = passesClippy "pineapple-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
