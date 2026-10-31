# The checks of the Linux builder, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareDarwin,
  holds,
  lib,
  self,
  ...
}:
let
  # A Mac running the builder with these settings, and the VM's guest on it.
  macOf =
    settings:
    (bareDarwin [
      self.darwinModules.linux-builder
      { jitsusama.linux-builder = settings; }
    ]).config;
  guestOf = settings: (macOf settings).jitsusama.linux-builder.builder.nixosConfig;
  # The features the Mac's line offers.
  offered =
    settings:
    lib.splitString "," (
      builtins.elemAt (lib.splitString " "
        (macOf settings).environment.etc."nix/linux-builder.conf".text
      ) 7
    );

  # The guest nixpkgs builds, which is the one its cache holds.
  cached = (bareDarwin [ ]).pkgs.darwin.linux-builder-vz.nixosConfig;

  another = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGRkdxraohorYagcirm3lEonh6at0F7DvFoLuEqBQajh elsewhere";
  plain = guestOf { };
  opened = guestOf { authorizedKeys = [ another ]; };
  marked = guestOf { features = [ "own-code" ]; };
  system = guest: builtins.unsafeDiscardStringContext guest.system.build.toplevel.drvPath;
in
{
  # A key from another machine lets it in to the VM's builder, where the
  # guest's SSH reads it. Without one, the guest is nixpkgs' own, sized, so
  # it still comes from the cache.
  linux-builder-lets-in-only-the-keys-it-is-given =
    holds "linux-builder-lets-in-only-the-keys-it-is-given"
      "The VM's builder didn't take exactly the keys it was given, or a VM given none isn't the cached one."
      (
        lib.hasInfix another opened.environment.etc."ssh/authorized_keys.d/builder".source.text
        && builtins.elem "/etc/ssh/authorized_keys.d/%u" opened.services.openssh.authorizedKeysFiles
        && builtins.length opened.services.openssh.authorizedKeysFiles == 2
        && plain.services.openssh.authorizedKeysFiles == cached.services.openssh.authorizedKeysFiles
        && system plain == system cached
        && system opened != system cached
      );

  # A feature of the machine's own is offered on the Mac's line and by the
  # guest's Nix, beside the builder's own, so a piece asking for it is both
  # sent there and built there; with none, both offer only the builder's.
  linux-builder-offers-the-features-it-is-given =
    holds "linux-builder-offers-the-features-it-is-given"
      "The VM's builder didn't offer exactly benchmark, big-parallel and the features it was given, on the Mac's line and in the guest's Nix."
      (
        offered { } == [
          "benchmark"
          "big-parallel"
        ]
        &&
          offered { features = [ "own-code" ]; } == [
            "benchmark"
            "big-parallel"
            "own-code"
          ]
        && marked.nix.settings.extra-system-features == [ "own-code" ]
        && !(plain.nix.settings ? extra-system-features)
        && builtins.elem "big-parallel" marked.nix.settings.system-features
      );
}
