{
  mkShell,
  rustc,
  rust-analyzer,
  cargo,
  rustfmt,
  clippy,
  npins,
  sources,
  pre-commit,
  pkg-config,
  udev,
  nixfmt-rfc-style,
  rustPlatform,
  writeShellScriptBin,
}:
mkShell {
  nativeBuildInputs = [
    rustc
    rust-analyzer
    cargo
    rustfmt
    clippy
    npins
    nixfmt-rfc-style
    pkg-config
    udev
    rustPlatform.rustLibSrc
    (writeShellScriptBin "update-vendored-deps" ''
      set +ex
      NIX_CARGO_CONFIG_DIR=${toString ./../.cargo}
      mv $NIX_CARGO_CONFIG_DIR/config.toml $NIX_CARGO_CONFIG_DIR/config.toml.bak
      cd $NIX_CARGO_CONFIG_DIR/.. && (cargo update && cargo vendor --versioned-dirs)
      mv $NIX_CARGO_CONFIG_DIR/config.toml.bak $NIX_CARGO_CONFIG_DIR/config.toml
    '')
  ];
  env = {
    NIX_PATH = "nixpkgs=${sources.nixpkgs}";
    NPINS_DIRECTORY = toString ./npins;
    #RA_LOG="info";
    #RA_LOG_FILE = "/tmp/ra.log";
    #RUST_SRC_PATH = "${rustPlatform.rustLibSrc}/lib/rustlib/src/rust/library";
  };

  inherit (pre-commit) shellHook;
}
