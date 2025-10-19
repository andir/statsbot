{ lib, workspaceToml, ... }:
let
  paths = (
    lib.flatten (
      map (crate: [
        "^/${crate}$"
        "^/${crate}/src$"
        "^/${crate}/src/.+$"
        "^/${crate}/Cargo.toml$"
      ]) workspaceToml.workspace.members
    )
    ++ [
      "^/Cargo.toml$"
      "^/Cargo.lock$"
      "^/vendor$"
      "^/vendor/.+$"
      "^/.cargo$"
      "^/.cargo/.+$"
    ]
  );

  extractSource =
    src:
    let
      baseDir = toString src;
    in
    expressions:
    builtins.path {
      path = src;
      filter =
        path:
        let
          suffix = lib.removePrefix baseDir path;
        in
        _: lib.any (r: builtins.match r suffix != null) expressions;
      name = "source";
    };
in
extractSource ./.. paths
