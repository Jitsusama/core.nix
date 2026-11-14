# The reader, built from this directory, its tests run as it builds, with
# MiTeX for its equations, a fonts folder of the faces its page sets and
# no more, since scanning every face a family ships costs a cold start, and
# wl-copy, which its copies go to the clipboard through. Its dependencies
# build once, apart from it (../cargo/dependencies.nix), so a change to it
# or to the key language compiles only its own crate. It takes only the
# key language's acts it reads, so a change to a list's act, the language's
# most common, builds nothing of it. It takes its shape from the terminal
# kit (../terminal-kit), whose source both builds take, since cargo builds
# it as one of the dependencies.
{
  callPackage,
  copyDesktopItems,
  fetchzip,
  fontconfig,
  formats,
  fraunces,
  language ? ../keys/language.toml,
  lib,
  libGL,
  libxkbcommon,
  makeDesktopItem,
  makeWrapper,
  nerd-fonts,
  pkg-config,
  runCommand,
  rustPlatform,
  wayland,
  wl-clipboard,
}:
let
  # MiTeX, LaTeX's maths for Typst, as Typst's own package index gives it.
  mitex = fetchzip {
    url = "https://packages.typst.org/preview/mitex-0.2.7.tar.gz";
    hash = "sha256-BG4csWQ2NjjXFIg+mHpTcdXURkR29x8eNugpiTwZpJ0=";
    stripRoot = false;
  };

  # The acts a page holds, and those of no kind, all the reader reads of
  # the language (src/keys/mod.rs). Written from what's left, the same
  # whatever another kind's acts say, so its build stays as it was.
  inherit (builtins.fromTOML (builtins.readFile language)) acts;
  spoken = (formats.toml { }).generate "language.toml" {
    acts = lib.filterAttrs (_: act: !(act ? kinds) || lib.elem "page" act.kinds) acts;
  };

  # One folder of one family's faces, for the theme's two the page sets:
  # the serif as its two variable files, upright and italic, which the page
  # sets at its own axes, without the static cuts that share their family
  # name (../fonts/system.nix says why); and the mono face as the terminal
  # has it, without its narrower and proportional cuts.
  faces =
    name: package: glob:
    runCommand name { } ''
      mkdir -p $out
      ln -s ${package}/share/fonts/${glob} $out/
    '';
  fonts = [
    (faces "fraunces-variable" fraunces "truetype/*'[SOFT,WONK,opsz,wght].ttf'")
    (faces "monaspace-xenon" nerd-fonts.monaspace "opentype/NerdFonts/*/MonaspiceXeNerdFont-*.otf")
  ];
in
(callPackage ../cargo/dependencies.nix { }).build rustPlatform
  (lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  })
  {
    pname = "reader";
    version = "0.1.0";
    # The page's part of the key language beside it, which its keys are
    # read from as it builds.
    src = runCommand "source" { } ''
      cp -r ${
        lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./src
            ../terminal-kit/Cargo.toml
            ../terminal-kit/src
          ];
        }
      } $out
      chmod -R u+w $out
      mkdir $out/keys
      cp ${spoken} $out/keys/language.toml
    '';
    sourceRoot = "source/reader";
    cargoLock.lockFile = ./Cargo.lock;
    nativeBuildInputs = [
      copyDesktopItems
      makeWrapper
      pkg-config
    ];
    buildInputs = [
      fontconfig
      libGL
      libxkbcommon
      wayland
    ];
    # The page's tests set pages, which takes MiTeX.
    preCheck = ''
      export READER_MITEX=${mitex}
    '';
    desktopItems = [
      (makeDesktopItem {
        name = "reader";
        desktopName = "Reader";
        comment = "Read a markdown file as a page";
        exec = "reader %f";
        icon = "text-markdown";
        mimeTypes = [ "text/markdown" ];
        categories = [
          "Office"
          "Viewer"
        ];
      })
    ];
    postInstall = ''
      wrapProgram $out/bin/reader \
        --set READER_MITEX ${mitex} \
        --set READER_FONTS ${lib.concatStringsSep ":" fonts} \
        --set READER_COPY ${lib.getExe' wl-clipboard "wl-copy"}
    '';
    passthru = { inherit fonts mitex; };
    meta = {
      description = "Reads a markdown file as a page, set by Typst and scrolled on the GPU, with Neovim's keys";
      license = lib.licenses.mit;
      mainProgram = "reader";
      # It draws through Wayland and EGL.
      platforms = lib.platforms.linux;
    };
  }
