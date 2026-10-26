# FIDO2 keys standing in for YubiKeys in the guide's VM tests: QEMU built
# with canokey, state files for keys made before QEMU starts, and set-pin to
# give a fresh key its PIN. canokey keeps one global state, so a second
# device crashes it, and a state file made on a hot-plug answers every
# makeCredential with 0xF1, so a test swaps one device between state files
# made first (spikes in QEST-20261011-2HN63P). Its random numbers come from a
# fixed seed, which would give every key the same secrets, so a
# getrandom-backed random32 takes its place. It's written here, as
# canokey-qemu's postPatch is extended, rather than in a file of its own,
# which would change the patch's text and so rebuild QEMU for nothing.
{ pkgs }:
let
  inherit (pkgs) lib;
  canokey = pkgs.canokey-qemu.overrideAttrs (old: {
    postPatch = old.postPatch + ''
      cat >> Src/qemu.c <<'EOF'

      #include <sys/random.h>
      uint32_t random32(void) {
        uint32_t r;
        if (getrandom(&r, sizeof(r), 0) != sizeof(r)) abort();
        return r;
      }
      EOF
    '';
  });
  qemu = pkgs.qemu_test.override {
    canokeySupport = true;
    canokey-qemu = canokey;
  };
in
{
  inherit qemu;

  # A state file for each key named, made by this QEMU at its start.
  keys =
    names:
    pkgs.runCommand "canokey-keys" { nativeBuildInputs = [ qemu ]; } ''
      mkdir $out
      for name in ${lib.escapeShellArgs names}; do
        echo quit | qemu-system-x86_64 -machine q35 -accel tcg -nodefaults -display none \
          -S -monitor stdio -device qemu-xhci,id=usb -device canokey,bus=usb.0,file=$out/$name
        test -s $out/$name
      done
    '';

  # Gives the one key plugged in a PIN, as a person's YubiKeys have one
  # (set-pin.py), held to ruff and mypy through `nix fmt` rather than the
  # writers' flake8.
  setPin = pkgs.writers.writePython3Bin "set-pin" {
    libraries = [ pkgs.python3Packages.fido2 ];
    doCheck = false;
  } (builtins.readFile ./set-pin.py);
}
