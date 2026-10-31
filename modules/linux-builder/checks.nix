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
  # The VM's guest on a Mac running the builder with these settings.
  guestOf =
    settings:
    (bareDarwin [
      self.darwinModules.linux-builder
      { jitsusama.linux-builder = settings; }
    ]).config.jitsusama.linux-builder.builder.nixosConfig;

  # The guest nixpkgs builds, which is the one its cache holds.
  cached = (bareDarwin [ ]).pkgs.darwin.linux-builder-vz.nixosConfig;

  another = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGRkdxraohorYagcirm3lEonh6at0F7DvFoLuEqBQajh elsewhere";
  plain = guestOf { };
  opened = guestOf { authorizedKeys = [ another ]; };
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
}
