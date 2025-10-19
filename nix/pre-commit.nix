{
  pkgs,
  sources,
  filteredSource,
  ...
}:
(import sources."pre-commit-hooks.nix").run {
  src = filteredSource;
  hooks = {
    nixfmt-rfc-style = {
      enable = true;
      package = pkgs.nixfmt-rfc-style;
    };
    rustfmt.enable = true;
    clippy = {
      enable = true;
      packageOverrides = {
        inherit (pkgs) clippy cargo;
      };
      settings.allFeatures = true;
    };
  };
}
