{
  system ? builtins.currentSystem,
}:
let
  p = import ./nix { inherit system; };
in
p.package
