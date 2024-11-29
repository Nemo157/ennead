{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
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
      };
    }
  );
}
