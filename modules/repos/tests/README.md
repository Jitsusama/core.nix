# Repos' Tests

The tests that reach beyond the crate's sources: Cargo runs the Rust one as
repos builds, in `repos-reads-the-disk`, and repos' checks (../checks.nix)
run the script.

- `serve.rs` runs `repos serve` as its unit runs it, over a checkout laid
  out by hand as git lays it out, and holds that a watch is told when its
  branch moves and when a tree is cut from it.
- `repos-is-told-every-realms-owners.sh` reads the refresh's unit as
  home-manager writes it, for `repos-is-told-every-realms-owners`, and
  fails unless every realm's owners reach repos as one word.
