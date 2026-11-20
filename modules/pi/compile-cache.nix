# Where a pi started from a shell keeps node's compiled-code cache
# (NODE_COMPILE_CACHE, not the glossary's compile cache, which is
# sccache's). node reads it back as compiled code when pi next starts,
# outside the agents' fence. Left to itself it keeps the cache in the
# temporary folder, which a fenced command can write, so a pi started
# from a shell compiles into pi's state, which the fence holds read-only.
# The harness names the same folder for every pi it starts itself, and a
# pi host keeps its own cache in pi's state too.
#
# It is the shell's alone, never the user manager's: the node takes a pi
# host's word on its sessions only from a host given no
# NODE_COMPILE_CACHE, and every host is a user unit. So a pi the launcher
# starts through a host gets the folder from the harness, and one launchd
# starts on a Mac gets it the same way.
{ config, ... }:
{
  home.sessionVariables.NODE_COMPILE_CACHE = "${config.xdg.stateHome}/pi/agentic-harness.pi/node-compile-cache";
}
