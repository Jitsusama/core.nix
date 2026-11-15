# pi as a host runs it, built from pi's own source with every host patch,
# applied exactly, never fuzzed, so a pi that moved under them fails a check
# rather than in a host. Every pi an account starts stays unpatched: only
# the checks that run a host build this one, pi's own beside this and the VM
# test that runs a realm's host through a reboot. pi's build names its package
# folder under lib/node_modules with a word stands-alone refuses outside a
# checks.nix, so each caller, a checks.nix, gives it.
{ pkgs, folder }:
pkgs.pi-coding-agent.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [
    ./host-patches/pi-host-agent-interface.patch
    ./host-patches/pi-host-committed-turn.patch
    ./host-patches/pi-host-interactive-lifecycle.patch
    ./host-patches/pi-host-model-auth-context.patch
    ./host-patches/pi-host-extension-cache-per-folder.patch
    ./host-patches/pi-host-extension-cache-keyed-shared.patch
    ./host-patches/pi-host-mcp-headers-once-a-burst.patch
    ./host-patches/pi-host-models-cloned-once.patch
    ./host-patches/pi-host-tools-found-once.patch
    ./host-patches/pi-host-extension-aliases-sorted-once.patch
    ./host-patches/pi-host-boundary-context-when-read.patch
  ];
  patchFlags = [
    "-p1"
    "--fuzz=0"
  ];
  # nixpkgs keeps the libraries pi's command loads and drops pi-schema, which
  # the durable runtime a host drives imports through chord.
  postInstall = old.postInstall + ''
    cp -r packages/schema "$out/lib/node_modules/${folder}/node_modules/@earendil-works/pi-schema"
  '';
})
