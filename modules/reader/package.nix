# The reader, built from this directory, its tests run as it builds, with
# MiTeX for its equations and a fonts folder of the faces its page sets and
# no more, since scanning every face a family ships costs a cold start.
{
  copyDesktopItems,
  fetchzip,
  fontconfig,
  fraunces,
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
}:
let
  # MiTeX, LaTeX's maths for Typst, as Typst's own package index gives it.
  mitex = fetchzip {
    url = "https://packages.typst.org/preview/mitex-0.2.7.tar.gz";
    hash = "sha256-BG4csWQ2NjjXFIg+mHpTcdXURkR29x8eNugpiTwZpJ0=";
    stripRoot = false;
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
rustPlatform.buildRustPackage {
  pname = "reader";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
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
      --set READER_FONTS ${lib.concatStringsSep ":" fonts}
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
