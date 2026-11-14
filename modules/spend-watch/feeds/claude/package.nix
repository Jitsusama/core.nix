# claude-plan-feed, spend-watch's feed for a Claude plan, built from this
# directory; its tests run as it builds, against recorded answers. It takes
# its shape from the terminal kit (../../../terminal-kit), so the source is
# cut from the modules folder holding both.
{ lib, rustPlatform }:
rustPlatform.buildRustPackage rec {
  pname = "claude-plan-feed";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../../terminal-kit/Cargo.toml
      ../../../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/spend-watch/feeds/claude";
  cargoLock.lockFile = ./Cargo.lock;
  meta = {
    description = "Says how much of a Claude plan's five hours and week is used, for spend-watch";
    license = lib.licenses.mit;
    mainProgram = "claude-plan-feed";
    # Claude Code keeps its sign-in in a file only on Linux; on macOS it's
    # in the keychain, and spend-watch itself runs only on Linux.
    platforms = lib.platforms.linux;
  };
}
