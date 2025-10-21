{
  rustPlatform,
  filteredSource,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../statsbot/Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  src = filteredSource;

  cargoVendorDir = "vendor";
  cargoBuildFlags = [
    "--bin"
    cargoToml.package.name
  ];

}
