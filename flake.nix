{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, home-manager }:
  let
    systems = builtins.filter
      (system: nixpkgs.lib.strings.hasSuffix "linux" system)
      flake-utils.lib.defaultSystems;
  in flake-utils.lib.eachSystem systems (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };

      rust-toolchain  = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
        toolchain.minimal.override {
          targets = [
            "thumbv6m-none-eabi"
          ];
          extensions = [
            "rust-src"
            "clippy"
            "rustfmt"
          ];
        }
      );
    in {
      packages."ἐννεάς-cli" = pkgs.callPackage ./nix/package.nix {
        rustPlatform = pkgs.makeRustPlatform {
          rustc = pkgs.rust-bin.selectLatestNightlyWith (t: t.minimal);
          cargo = pkgs.rust-bin.selectLatestNightlyWith (t: t.minimal);
        };
      };

      checks.home-module = let
        hmConfig = home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          modules = [
            self.homeManagerModules.default
            {
              home.username = "test";
              home.homeDirectory = "/home/test";
              home.stateVersion = "24.11";
              services."ἐννεάς-listenbrainz-watcher" = {
                enable = true;
                username = "test";
              };
            }
          ];
        };
      in hmConfig.activationPackage;

      checks.home-module-device-activation = let
        hmConfig = home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          modules = [
            self.homeManagerModules.default
            {
              home.username = "test";
              home.homeDirectory = "/home/test";
              home.stateVersion = "24.11";
              services."ἐννεάς-listenbrainz-watcher" = {
                enable = true;
                username = "test";
                deviceActivation = true;
              };
            }
          ];
        };
      in hmConfig.activationPackage;

      checks.nixos-module = let
        nixosConfig = nixpkgs.lib.nixosSystem {
          inherit system;
          modules = [
            self.nixosModules.default
            {
              # Minimal config to evaluate the module
              boot.loader.grub.enable = false;
              fileSystems."/".device = "none";
              system.stateVersion = "24.11";
              hardware."ἐννεάς".enable = true;
            }
          ];
        };
      in nixosConfig.config.system.build.toplevel;

      devShells.default = with pkgs; mkShell {
        nativeBuildInputs = [
          elf2uf2-rs
          flip-link
          gcc-arm-embedded
          pkg-config
          rust-toolchain
        ];

        buildInputs = [
          udev
        ];

        ENNEAD_DEV = 1;
      };
    }
  ) // {
    homeManagerModules.default = import ./nix/home-module.nix self;
    nixosModules.default = import ./nix/nixos-module.nix;
  };
}
