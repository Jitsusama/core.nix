# git-engine, built from this directory's crate. Its tests run as it builds.
# It takes its help and facts from the terminal kit (../terminal-kit), so the
# source is cut from the folder holding both. The git built beside it says
# where it reads its system-wide config and attributes, so the engine reads
# them there too: under /etc on Linux, and in git's own folder on a Mac.
{
  git,
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "git-engine";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "source/git-engine";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ git ];
  # Said apart from the files named below, so the tests can tell the
  # package that stopped naming them from a build by cargo alone.
  env.GIT_ENGINE_PACKAGED = "1";
  # Each answer is kept apart first, so a git that can't answer fails the
  # build rather than naming an empty path.
  preBuild = ''
    config=$(env -u GIT_CONFIG_SYSTEM -u GIT_CONFIG_NOSYSTEM git var GIT_CONFIG_SYSTEM)
    attributes=$(env -u GIT_ATTR_SYSTEM -u GIT_ATTR_NOSYSTEM git var GIT_ATTR_SYSTEM)
    export GIT_ENGINE_CONFIG_SYSTEM=$config GIT_ENGINE_ATTR_SYSTEM=$attributes
  '';
  meta = {
    description = "Answers the prompt's questions about a git repository, starting nothing";
    license = lib.licenses.mit;
    mainProgram = "git-engine";
    platforms = lib.platforms.unix;
  };
}
