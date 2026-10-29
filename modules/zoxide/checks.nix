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

  # The account's zoxide lists the folder visited last first when asked to,
  # as the launcher's Recent asks, though another was visited more often,
  # and leaves its database as it was. Its folders come from z's list, which
  # gives each visit's time, since visits made here would share one second.
  zoxide-lists-the-folder-visited-last-first =
    let
      home = account workstation;
    in
    pkgs.runCommandLocal "zoxide-lists-the-folder-visited-last-first"
      { nativeBuildInputs = [ home.programs.zoxide.package ]; }
      ''
        export _ZO_DATA_DIR=$PWD/zoxide
        mkdir -p zoxide often lately
        printf '%s|100|1000000000\n%s|1|1000000600\n' $PWD/often $PWD/lately > z
        zoxide import --from z z
        cp zoxide/db.zo before
        ranked=$(zoxide query --list)
        [ "$ranked" = "$PWD/often"$'\n'"$PWD/lately" ] || { echo "zoxide ranked: $ranked"; exit 1; }
        listed=$(zoxide query --list --sort-by last-accessed)
        [ "$listed" = "$PWD/lately"$'\n'"$PWD/often" ] || { echo "zoxide listed by last visit: $listed"; exit 1; }
        cmp -s before zoxide/db.zo || { echo "listing by last visit wrote zoxide's database"; exit 1; }
        touch $out
      '';
}
