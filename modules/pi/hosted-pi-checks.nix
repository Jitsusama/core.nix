# The checks of pi as a host runs it, each running pi built with every
# host patch applied exactly, which pi's checks.nix gathers beside its own,
# kept apart since they change with hosted-pi.nix, host-patches and the
# tests they run alone. pi's build names its package folder with a word
# only a checks.nix may say, so pi's checks.nix gives it as folder.
{
  lib,
  pkgs,
  folder,
  ...
}:
let
  # pi as a host runs it, every host patch applied exactly (hosted-pi.nix).
  hostedPi = import ./hosted-pi.nix {
    inherit pkgs folder;
  };
in
{
  # A hosted pi's seams are there to detect, a turn its durable generation
  # commits is the host's to end or continue, and words the host's agent
  # queued show as pending until a dequeue clears them.
  pi-hosted-keeps-turns-its-own = pkgs.runCommandLocal "pi-hosted-keeps-turns-its-own" { } ''
    HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
      ${./tests/pi-hosted-keeps-turns-its-own.mjs} ${hostedPi}/lib/node_modules/${folder}
    touch $out
  '';

  # A hosted pi builds a turn's boundary context only when a handler reads
  # it, so observers no longer hold the main thread every conversation shares.
  pi-hosted-builds-boundary-context-when-read =
    pkgs.runCommandLocal "pi-hosted-builds-boundary-context-when-read" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-builds-boundary-context-when-read.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi freezes each context entry it reads without putting its
  # objects in a weak set, whose in-place rehash held a host's thread 95 s.
  pi-hosted-freezes-context-without-a-set =
    pkgs.runCommandLocal "pi-hosted-freezes-context-without-a-set" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-freezes-context-without-a-set.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi keeps a copy of each patch it applied, since a running
  # host's copies are what cadence reads before it tells a miss fixed.
  pi-hosted-keeps-the-patches-it-applied =
    pkgs.runCommandLocal "pi-hosted-keeps-the-patches-it-applied" { }
      ''
        ${lib.getExe pkgs.bash} ${./tests/pi-hosted-keeps-the-patches-it-applied.sh} \
          ${./host-patches} ${hostedPi} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A run a hosted pi's agent starts itself, carrying a conversation on with
  # no starting message, is prepared by extensions and settled by pi as a
  # prompt's run is, so a carried turn keeps every section they add. The
  # test imports the session it drives from the helper beside it.
  pi-hosted-runs-its-own-turns-through-pi =
    let
      tests = lib.fileset.toSource {
        root = ./tests;
        fileset = lib.fileset.unions [
          ./tests/hosted-pi-session.mjs
          ./tests/pi-hosted-runs-its-own-turns-through-pi.mjs
        ];
      };
    in
    pkgs.runCommandLocal "pi-hosted-runs-its-own-turns-through-pi" { } ''
      HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
        ${tests}/pi-hosted-runs-its-own-turns-through-pi.mjs ${hostedPi}/lib/node_modules/${folder}
      touch $out
    '';

  # A hosted pi loads a linked extension through where its link points now,
  # so once the link an extension is found through moves, a reload loads the
  # new copy and its own helpers, whether the old copy is gone or not.
  pi-hosted-loads-where-a-link-points =
    pkgs.runCommandLocal "pi-hosted-loads-where-a-link-points" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-loads-where-a-link-points.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi keeps each folder's extensions, so a conversation opening or
  # waking in a folder that loaded them doesn't evaluate their modules again,
  # and no two folders share one.
  pi-hosted-keeps-each-folders-extensions =
    pkgs.runCommandLocal "pi-hosted-keeps-each-folders-extensions" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-keeps-each-folders-extensions.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi shares a package's extensions across folders only when the
  # package declares its per-session state keyed, so a pairing another keeps
  # in a module never crosses from one folder's conversation to another's.
  pi-hosted-shares-only-keyed-packages =
    pkgs.runCommandLocal "pi-hosted-shares-only-keyed-packages" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-shares-only-keyed-packages.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi evaluates a keyed package's modules once for all its
  # extensions, in a cache of the package's own, while an unkeyed package's
  # extensions keep a copy each; a reload or a moved link lets go of it,
  # and a turn held too long lets loads go on without it. Only finished
  # modules are shared, so a failed load or an import made as an extension
  # runs leaves nothing part-way for another, which the second test drives.
  # Both import the loader they drive from the helper beside them.
  pi-hosted-shares-keyed-modules =
    let
      tests = lib.fileset.toSource {
        root = ./tests;
        fileset = lib.fileset.unions [
          ./tests/hosted-keyed-packages.mjs
          ./tests/pi-hosted-keyed-late-imports.mjs
          ./tests/pi-hosted-shares-keyed-modules.mjs
        ];
      };
    in
    pkgs.runCommandLocal "pi-hosted-shares-keyed-modules" { } ''
      HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
        ${tests}/pi-hosted-shares-keyed-modules.mjs ${hostedPi}/lib/node_modules/${folder}
      HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
        ${tests}/pi-hosted-keyed-late-imports.mjs ${hostedPi}/lib/node_modules/${folder}
      touch $out
    '';

  # A hosted pi's conversations each read their keys and headers from their
  # own auth context and never from the process, while a pi given none
  # reads the process as ordinary pi does.
  pi-hosted-keeps-each-conversations-keys =
    pkgs.runCommandLocal "pi-hosted-keeps-each-conversations-keys" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-keeps-each-conversations-keys.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # What each conversation's start repeats in a host's one process is done
  # once, or without holding it: an MCP server's header command, a tool
  # looked up on the path, and a second clone of each stored catalogue.
  pi-hosted-starts-conversations-lightly =
    pkgs.runCommandLocal "pi-hosted-starts-conversations-lightly" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-starts-conversations-lightly.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi loads extensions without sorting its alias map for every
  # module jiti evaluates, while the aliases resolve as before and each
  # folder's load still evaluates an extension's modules afresh.
  pi-hosted-loads-extensions-lightly =
    pkgs.runCommandLocal "pi-hosted-loads-extensions-lightly" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-loads-extensions-lightly.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi gives the event loop a turn before each extension's import
  # and each factory, so one conversation's load never holds the host's one
  # thread whole, and the extensions still load in the order they were given.
  pi-hosted-yields-between-extensions =
    pkgs.runCommandLocal "pi-hosted-yields-between-extensions" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-yields-between-extensions.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi says on stderr each extension whose handlers for one session
  # event, or resources_discover, took 250 ms or more, so a slow open names
  # the extension it waited on.
  pi-hosted-names-handlers-that-hold =
    pkgs.runCommandLocal "pi-hosted-names-handlers-that-hold" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-names-handlers-that-hold.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi draws its conversation before its extensions bind, so a
  # session_start that never settles leaves the window readable, not blank.
  pi-hosted-draws-before-it-binds = pkgs.runCommandLocal "pi-hosted-draws-before-it-binds" { } ''
    HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
      ${./tests/pi-hosted-draws-before-it-binds.mjs} ${hostedPi}/lib/node_modules/${folder}
    touch $out
  '';

  # A hosted pi's screen stays in its own conversation: quitting or a crash
  # ends only that screen and what it was doing, a stopped screen builds
  # nothing for an extension and disposes the footer, header and editor one
  # gave it, a suspend stops only the viewer that asked, every screen hears
  # a theme change, and ordinary pi keeps every path.
  pi-hosted-screen-stays-in-its-session =
    pkgs.runCommandLocal "pi-hosted-screen-stays-in-its-session" { }
      ''
        HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-screen-stays-in-its-session.mjs} ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';

  # A hosted pi's screen does in its viewers' terminals what pi does in its
  # own: a copy reaches them, a viewer who joins late is sent the images on
  # the screen, every frame is drawn within its terminal's render scope, so
  # as the conversation it shows, the external editor opens where the viewer
  # who asked is, a quit asks its host first, with the key that asked still
  # in hand, and
  # an extension's own component says it waits on the person, so its host
  # can let a viewer attach to answer it.
  pi-hosted-screen-reaches-its-viewers =
    pkgs.runCommandLocal "pi-hosted-screen-reaches-its-viewers" { }
      ''
        HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${./tests/pi-hosted-screen-reaches-its-viewers.mjs} \
          ${hostedPi}/lib/node_modules/${folder}
        touch $out
      '';
}
