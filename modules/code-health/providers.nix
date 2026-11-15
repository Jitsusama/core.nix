# The Terraform providers code-health readies a workspace with before
# tofu validate reads it (src/terraform_validate.rs), laid out as a tofu
# filesystem mirror, so it never asks a registry: google's newest 7.x
# release, as OpenTofu publishes it, for the systems this flake builds.
{
  fetchurl,
  linkFarm,
  stdenv,
}:
let
  version = "7.46.2";
  builds = {
    x86_64-linux = {
      platform = "linux_amd64";
      sha256 = "cf21367691194ca138f0f75d46f3fc5bd1afd6c9c54e774172552e272654e23e";
    };
    aarch64-darwin = {
      platform = "darwin_arm64";
      sha256 = "a49a11d051a785ce7b62ad1ded5eb17208d8609fcdea69e047e9c5ad98e07e5f";
    };
  };
  build = builds.${stdenv.hostPlatform.system};
  zip = "terraform-provider-google_${version}_${build.platform}.zip";
in
linkFarm "code-health-terraform-providers" [
  {
    name = "registry.opentofu.org/hashicorp/google/${zip}";
    path = fetchurl {
      url = "https://github.com/opentofu/terraform-provider-google/releases/download/v${version}/${zip}";
      inherit (build) sha256;
    };
  }
]
