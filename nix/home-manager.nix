{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.sysnotifier;
in
{
  options.services.sysnotifier = {
    enable = lib.mkEnableOption "sysnotifier";
    package = lib.mkPackageOption pkgs "sysnotifier" { };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.sysnotifier = {
      Install = {
        WantedBy = [ "graphical-session.target" ];
      };

      Unit = {
        Description = "Pulse Audio and Notification bridge";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };

      Service = {
        ExecStart = "${lib.getExe cfg.package}";
        Restart = "always";
        RestartSec = "10";
      };
    };
  };
}
