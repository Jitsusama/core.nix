# The checks of zoxide, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # The account's zoxide lists its folders without writing its database, so
  # what watches the file, as the launcher's Recent does, isn't sent round
  # again by its own reading; a visit still writes it.
  zoxide-lists-without-writing =
    let
      home = account workstation;
    in
    pkgs.runCommandLocal "zoxide-lists-without-writing"
      { nativeBuildInputs = [ home.programs.zoxide.package ]; }
      ''
        export _ZO_DATA_DIR=$PWD/zoxide
        mkdir -p zoxide often lately
        zoxide add $PWD/often
        zoxide add $PWD/often
        zoxide add $PWD/lately
        cp zoxide/db.zo before
        listed=$(zoxide query --list)
        [ "$listed" = "$PWD/often"$'\n'"$PWD/lately" ] || { echo "zoxide listed: $listed"; exit 1; }
        cmp -s before zoxide/db.zo || { echo "listing the folders wrote zoxide's database"; exit 1; }
        zoxide add $PWD/lately
        cmp -s before zoxide/db.zo && { echo "a visit no longer writes zoxide's database"; exit 1; }
        touch $out
      '';
}
