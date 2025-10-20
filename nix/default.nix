let
  sources = import ./npins;
in
{
  nixpkgs ? sources.nixpkgs,
  system ? builtins.currentSystem,
}:
let
  pkgs = import nixpkgs {
    inherit system;
    overlays = [
      (self: super: {
        rust-analyzer-unwrapped = super.rust-analyzer-unwrapped.overrideAttrs (
          {
            patches ? [ ],
            ...
          }:
          {
            # see https://github.com/andir/rust-analyzer-reproducer/
            patches = patches ++ [
              #   ./rust-analyzer.patch
              (pkgs.fetchpatch {
                url = "https://patch-diff.githubusercontent.com/raw/rust-lang/rust-analyzer/pull/20866.patch";
                hash = "sha256-gza1XTKbRJPDRyROCEjrZ0loCmR/WbDsYdin7zTiF6g=";
              })
            ];
          }
        );
      })
    ];
  };
in
pkgs.lib.makeScope pkgs.newScope (
  self: with self; {
    workspaceToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
    filteredSource = callPackage ./filtered-source.nix { };
    pre-commit = callPackage ./pre-commit.nix { };
    shell = callPackage ./shell.nix { };
    package = callPackage ./package.nix { };
    inherit pkgs sources;
    inherit (pkgs) lib;
  }
)
