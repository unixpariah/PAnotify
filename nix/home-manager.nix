{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.panotify;
in
{
  options.services.panotify = {
    enable = lib.mkEnableOption "panotify";
    package = lib.mkPackageOption pkgs "panotify" { };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.panotify = {
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
