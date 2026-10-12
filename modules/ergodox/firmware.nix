# The ErgoDox EZ Glow's firmware, built from ZSA's QMK with the layout in
# keymap/: Oryx's source for the layout, as Oryx gave it, with a serial
# number of its own. It holds ergodox.hex, which zapp writes to the board;
# layout.json, QMK's own reading of keymap.c, which is what the shell draws
# the layers from; and serial, what the board answers once it runs this
# build.
#
# Import this with nixpkgs, then give it ZSA's QMK and the LUFA and printf
# that QMK's build for an AVR board compiles in.
{
  lib,
  stdenvNoCC,
  pkgsCross,
  python3Packages,
  qmk,
  which,
  ...
}:
{
  firmware,
  lufa,
  printf,
}:
let
  keymap = builtins.path {
    path = ./keymap;
    name = "ergodox-keymap";
  };

  # The keymap's own hash, so the serial changes whenever the layout or its
  # code does and stays put when they don't. Oryx's serials name a layout and
  # a revision, which is how Keymapp and `zapp update` find a board's layout
  # to replace it; this one names nothing Oryx holds.
  serial = "jitsusama/${builtins.substring 11 8 keymap}";

  # QMK's command, which its build runs to read keyboard definitions, with
  # only its Python. nixpkgs' carries the compilers and flashers for every
  # board QMK supports, an ARM toolchain among them; this board needs the
  # AVR compiler alone. ZSA's QMK also wants three packages nixpkgs' doesn't.
  cli = qmk.overridePythonAttrs (old: {
    propagatedBuildInputs = [ ];
    dependencies =
      old.dependencies
      ++ (with python3Packages; [
        appdirs
        argcomplete
        colorama
      ]);
  });

  # The Glow is one of the boards QMK builds from the ErgoDox EZ's ATmega32U4
  # definitions, which keep their keymaps for every one of them.
  board = "zsa/ergodox_ez/m32u4/glow";
  name = "jitsusama";
  keymapIn = "keyboards/zsa/ergodox_ez/m32u4/keymaps/${name}";
in
stdenvNoCC.mkDerivation {
  pname = "ergodox-firmware";
  version = builtins.substring 0 7 firmware.rev;
  src = firmware;

  patches = [ ./unused-config-result.patch ];

  # QMK's submodules aren't in GitHub's archive of it, and its build finds a
  # keymap by the folder it sits in.
  postUnpack = ''
    rm -rf $sourceRoot/lib/lufa $sourceRoot/lib/printf
    cp -r ${lufa} $sourceRoot/lib/lufa
    cp -r ${printf} $sourceRoot/lib/printf
    mkdir -p $(dirname $sourceRoot/${keymapIn})
    cp -r ${keymap} $sourceRoot/${keymapIn}
    chmod -R u+w $sourceRoot/lib $sourceRoot/keyboards
  '';

  nativeBuildInputs = [
    cli
    which
    pkgsCross.avr.buildPackages.gcc
    pkgsCross.avr.buildPackages.binutils
  ];

  # Unpacked from an archive there's no repository for the build to ask its
  # version of. QMK's command keeps its settings in the home folder.
  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR QMK_HOME=$PWD
    make ${board}:${name} SKIP_GIT=yes EXTRAFLAGS='-DSERIAL_NUMBER=\"${serial}\"'
    qmk c2json --no-cpp -kb ${board} -km ${name} -o layout.json ${keymapIn}/keymap.c
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    install -D -m 444 ${lib.replaceStrings [ "/" ] [ "_" ] board}_${name}.hex $out/ergodox.hex
    install -m 444 layout.json $out/layout.json
    printf '%s\n' '${serial}' > $out/serial
    runHook postInstall
  '';

  passthru = { inherit serial; };
}
