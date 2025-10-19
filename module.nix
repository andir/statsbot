# untested but you can get the idea..
{
  lib,
  config,
  system,
  ...
}:
let
  cfg = config.service.statsbot;
  p = import ./nix { inherit system; };
in
with lib;
{
  options.services.statsbot = {
    enable = mkEnabelOption "enable statsbot";
    settings = mkOption {
      default = { };
      type = types.attrset;
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.statsbot =
      let
        configFile = (pkgs.writeText "config.json" cfg.settings).overrideAttrs (_: {
          checkPhase = ''
            ${p.package}/bin/statsbot check --config-file $out 
          '';
        });
      in
      {
        script = ''
          exec ${p.package}/bin/statsbot run --config-file ${configFile}
        '';
        serviceConfig = {
          User = "statsbot";
          DynamicUser = true;
        };
      };
  };
}
