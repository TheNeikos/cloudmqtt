{
  description = "The CloudMQTT Rust library";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-22.05";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        unstableRustTarget = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "miri" ];
        });

        craneLib = (crane.mkLib pkgs).overrideToolchain rustTarget;
        unstableCraneLib = (crane.mkLib pkgs).overrideToolchain unstableRustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };
        inherit (tomlInfo) pname version;
        src = ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        cloudmqtt = craneLib.buildPackage {
          inherit cargoArtifacts src version;
        };

        xargo = pkgs.rustPlatform.buildRustPackage
          rec {
            pname = "xargo";
            version = "0.3.26";

            src = pkgs.fetchFromGitHub {
              owner = "japaric";
              repo = pname;
              rev = "v${version}";
              hash = "sha256-MPopR58EIPiLg79wxf3nDy6SitdsmuUCjOLut8+fFJ4=";
            };

            cargoHash = "sha256-LmOu7Ni6TkACHy/ZG8ASG/P2UWEs3Qljz4RGSW1i3zk=";

            doCheck = false;

            buildInputs = [ ];
            nativeBuildInputs = [ ];
          };

      in
      rec {
        checks = {
          inherit cloudmqtt;

          cloudmqtt-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          cloudmqtt-fmt = craneLib.cargoFmt {
            inherit src;
          };

          cloudmqtt-miri = unstableCraneLib.cargoBuild {
            inherit src;
            pnameSuffix = "-miri";
            cargoArtifacts = null;
            cargoVendorDir = null;
            doRemapSourcePathPrefix = false;

            cargoBuildCommand = "cargo miri test --workspace --offline";
            doCheck = false;

            nativeBuildInputs = [ xargo ];

            preBuild = ''
              mkdir -p home
              cd home
              mkdir -p ''${CARGO_TARGET_DIR:-target}
              export HOME="$(pwd)"
            '';
            XARGO_RUST_SRC = "${unstableRustTarget}/lib/rustlib/src/rust/library";
            RUST_BACKTRACE = 1;
          };
        };

        apps.cloudmqtt = flake-utils.lib.mkApp {
          name = "cloudmqtt";
          drv = cloudmqtt;
        };
        apps.default = apps.cloudmqtt;

        devShells.default = devShells.cloudmqtt;
        devShells.cloudmqtt = pkgs.mkShell {
          buildInputs = [
          ];

          nativeBuildInputs = [
            unstableRustTarget
            xargo

            pkgs.cargo-msrv
            pkgs.cargo-deny
            pkgs.cargo-expand
            pkgs.cargo-bloat
          ];
        };
      }
    );
}
