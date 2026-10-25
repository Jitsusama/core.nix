# nixpkgs' settings on every machine, NixOS and nix-darwin alike.
{
  # Claude Code is unfree. Broken and insecure packages stay refused.
  nixpkgs.config.allowUnfree = true;
}
