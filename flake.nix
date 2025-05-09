{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    # The version of wasm-bindgen-cli needs to match the version in Cargo.lock
    nixpkgs-for-wasm-bindgen.url = "github:NixOS/nixpkgs/57d6973abba7ea108bac64ae7629e7431e0199b6";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, nixpkgs-for-wasm-bindgen, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLib = ((crane.mkLib pkgs).overrideToolchain rustToolchain).overrideScope (_final: _prev: {
          inherit (import nixpkgs-for-wasm-bindgen { inherit system; }) wasm-bindgen-cli;
        });

        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (lib.hasSuffix "\.html" path) ||
            (lib.hasSuffix "\.scss" path) ||
            (lib.hasInfix "/assets/" path) ||
            (craneLib.filterCargoSources path type)
          ;
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";

          nativeBuildInputs = (with pkgs; [
            pkg-config
            clang
            mold
            openssl
          ]);

          buildInputs = (with pkgs; [
            libxkbcommon
            alsa-lib
            udev
            vulkan-loader
            wayland
          ]) ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };

        # Build just the cargo dependencies, so we can reuse all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          # You cannot run cargo test on a wasm build
          doCheck = false;
        });

        # Build the actual crate itself, reusing the dependency artifacts from above.
        # This derivation is a directory you can put on a webserver.
        my-app = craneLib.buildTrunkPackage (commonArgs // {
          inherit cargoArtifacts;
          # The version of wasm-bindgen-cli here must match the one from Cargo.lock.
          wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
            version = "0.2.92";
            hash = "sha256-1VwY8vQy7soKEgbki4LD+v259751kKxSxmo/gqE6yV0=";
            cargoHash = "sha256-aACJ+lYNEU8FFBs158G1/JG8sc6Rq080PeKCMnwdpH0=";
          };
        });

        serve-app = pkgs.writeShellScriptBin "serve-app" ''
          ${pkgs.python3Minimal}/bin/python3 -m http.server --directory ${my-app} 8000
        '';
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-app;

          my-app-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Check formatting
          my-app-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages.default = my-app;

        # nix run .#serve
        apps.serve = flake-utils.lib.mkApp {
          drv = serve-app;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          packages = [
            pkgs.trunk
          ];
        };
      });
}
