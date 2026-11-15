# The polkit rules that keep an agent's process, wherever agents' work
# runs (agents-ask-first.rules names each place), from putting a prompt
# on Joel's screen:
# at 00:10 on 12
# November something asked for libvirt's system connection, and a password
# box sat there two minutes for an act he never started. What would ask is
# refused instead, so the agent hears why at once. first turns every later
# rule's asking into a refusal for them, so nixos.nix puts it before every
# other rule; last refuses what polkit's defaults would decide, so it goes
# after them all, and what a rule grants unasked (the work slices' weights,
# a switch) agents keep. One ask is put to Joel rather than refused: an
# agent running a command as root with run0 or pkexec, once per command
# while he's at the screen, since a root step he has to type himself
# stops him getting things done. The rules read a process's cgroup with
# cat and its command with ps, and write each refusal and ask to the
# journal with logger.
{
  coreutils,
  procps,
  util-linux,
}:
let
  tools =
    builtins.replaceStrings
      [ "@cat@" "@logger@" "@ps@" ]
      [
        "${coreutils}/bin/cat"
        "${util-linux}/bin/logger"
        "${procps}/bin/ps"
      ];
in
{
  first = tools (builtins.readFile ./agents-ask-first.rules);
  last = builtins.readFile ./agents-ask-last.rules;
}
