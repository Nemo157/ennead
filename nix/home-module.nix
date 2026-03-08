self:

{ config, lib, pkgs, ... }:

let
  cfg = config.services."ἐννεάς-listenbrainz-watcher";

  wrappedScript = pkgs.writeShellApplication {
    name = "ἐννεάς-listenbrainz-watcher";
    runtimeInputs = [ pkgs.curl pkgs.jq cfg.package ]
      ++ lib.optional cfg.notifyMissingCoverArt pkgs.libnotify;
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

    deviceActivation = lib.mkEnableOption "USB device activation (requires NixOS module)";

    notifyMissingCoverArt = lib.mkEnableOption "desktop notifications when cover art is not found";
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services."ennead-listenbrainz-watcher" = lib.mkMerge [
      {
        Unit.Description = "ἐννεάς ListenBrainz watcher";

        Service = {
          ExecStart = "${lib.getExe wrappedScript} ${cfg.username} coverartarchive";
          Restart = "on-failure";
          RestartSec = 30;
        };
      }
      (lib.mkIf cfg.notifyMissingCoverArt {
        Service.Environment = "NOTIFY_MISSING_ART=1";
      })
      (lib.mkIf cfg.deviceActivation {
        Unit.BindsTo = [ "dev-ennead.device" ];
        Unit.After = [ "dev-ennead.device" ];
      })
      (lib.mkIf (!cfg.deviceActivation) {
        Install.WantedBy = [ "default.target" ];
      })
    ];
  };
}
