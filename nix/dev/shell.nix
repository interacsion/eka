let
  inherit (atom) pkgs;
  toolchain = atom.fenix.fromToolchainFile { file = "${mod}/rust-toolchain.toml"; };
in
pkgs.mkShell {
  RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
  packages = with pkgs; [
    treefmt
    npins
    nixfmt-rfc-style
    shfmt
    taplo
    nodePackages.prettier
    toolchain
  ];
}
