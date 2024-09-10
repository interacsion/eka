let
  inherit (import ./npins) atom;
  fromManifest = import "${atom}/src/core/fromManifest.nix";

in
{
  env = fromManifest { } ./env.toml;
}
