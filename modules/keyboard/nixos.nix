# Which keyboard is the machine's own, so a module can treat it apart from
# any keyboard plugged in. The hardware says; nothing here changes a key.
{ lib, ... }:
{
  options.jitsusama.keyboard.builtIn = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "0001:0001" ];
    description = ''
      The built-in keyboard's IDs, as keyd names devices: vendor and product
      in hexadecimal, which `keyd monitor` prints. Empty for a machine with no
      keyboard of its own.
    '';
  };
}
