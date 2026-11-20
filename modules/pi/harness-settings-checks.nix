# The check of the harness's declared settings, which pi's checks.nix gathers
# beside its own, kept apart since it changes with harness-settings.nix and
# nothing else.
{
  account,
  bareDarwin,
  lib,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # The harness's config gets the settings a machine declares for it at
  # every switch, beside what the harness and a person wrote there; a
  # machine declaring none has its file left alone.
  pi-harness-takes-its-settings =
    let
      settings.sections.quest-workflow.budgetWhenCapped = "a-budget";
      home = account (
        workstation.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.pi.harness = { inherit settings; }; } ];
        }
      );
      program = lib.getExe (import ./settings.nix { inherit pkgs settings; });
      activated = lib.getExe (
        import ./settings.nix {
          inherit (workstation) pkgs;
          inherit settings;
        }
      );
      script = home.home.activation.piHarnessTakesItsSettings.data or "";
      # A Mac builds its own program, so only the file it names is held.
      mac = account (bareDarwin [
        self.darwinModules.workstation
        { home-manager.users.someone.jitsusama.pi.harness = { inherit settings; }; }
      ]);
      macScript = mac.home.activation.piHarnessTakesItsSettings.data or "";
      file = "/.config/pi/agentic-harness.pi/config.json";
    in
    assert lib.assertMsg (
      lib.hasInfix (builtins.unsafeDiscardStringContext activated) script && lib.hasInfix file script
    ) "The harness's activation doesn't run the settings program over its config file.";
    assert lib.assertMsg (
      lib.hasInfix "pi-settings" macScript && lib.hasInfix "/Users/someone${file}" macScript
    ) "A Mac's activation doesn't run the settings program over the harness's config file.";
    assert lib.assertMsg (lib.hasInfix "|| :" script)
      "A harness config left not JSON would stop the rest of the switch.";
    assert lib.assertMsg (
      !(account workstation).home.activation ? piHarnessTakesItsSettings
    ) "A machine declaring no harness settings still has its config written.";
    pkgs.runCommandLocal "pi-harness-takes-its-settings" { nativeBuildInputs = [ pkgs.jq ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/pi-harness-takes-its-settings.sh} ${program}
      touch $out
    '';
}
