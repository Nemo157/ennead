{ config, lib, pkgs, ... }:

let
  cfg = config.hardware."ἐννεάς";

  # Can't use extraRules because this needs to go before 73 for uaccess support
  udevRules = pkgs.writeTextFile {
    name = "ἐννεάς-udev-rules";
    text = ''
      # ID f055:cf82 Nullus157 ἐννεάς
      SUBSYSTEM=="usb", ENV{ID_VENDOR_ID}=="f055", ENV{ID_MODEL_ID}=="cf82", \
        TAG+="uaccess", MODE="660", \
        TAG+="systemd", \
        SYMLINK+="ennead", \
        ENV{SYSTEMD_ALIAS}+="/dev/ennead", \
        ENV{SYSTEMD_USER_WANTS}+="ennead-listenbrainz-watcher.service"
    '';
    destination = "/etc/udev/rules.d/70-ἐννεάς.rules";
  };
in {
  options.hardware."ἐννεάς" = {
    enable = lib.mkEnableOption "ἐννεάς USB device activation";
  };

  config = lib.mkIf cfg.enable {
    services.udev.packages = [ udevRules ];
  };
}
