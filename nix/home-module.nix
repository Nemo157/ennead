self:

{ config, lib, pkgs, ... }:

let
  cfg = config.services."ἐννεάς-listenbrainz-watcher";

  wrappedScript = pkgs.writeShellApplication {
    name = "ἐννεάς-listenbrainz-watcher";
    runtimeInputs = [ pkgs.curl pkgs.jq cfg.package ];
    text = builtins.readFile ../listenbrainz-watcher.sh;
    checkPhase = "";
  };
in {
  options.services."ἐννεάς-listenbrainz-watcher" = {
    enable = lib.mkEnableOption "ἐννεάς ListenBrainz watcher";

    username = lib.mkOption {
      type = lib.types.str;
      description = "ListenBrainz username to watch.";
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}."ἐννεάς-cli";
      description = "The ἐννεάς-cli package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services."ἐννεάς-listenbrainz-watcher" = {
      Unit.Description = "ἐννεάς ListenBrainz watcher";

      Service = {
        ExecStart = "${lib.getExe wrappedScript} ${cfg.username} coverartarchive";
        Restart = "on-failure";
        RestartSec = 30;
      };

      Install.WantedBy = [ "default.target" ];
    };
  };
}
