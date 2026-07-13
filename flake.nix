{
  description = "fff-cli: terminal frontend for fff-search";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    forAllSystems = nixpkgs.lib.genAttrs systems;

    mkPackage = pkgs:
      pkgs.rustPlatform.buildRustPackage {
        pname = "fff-cli";
        version = "0.1.0";

        src = pkgs.lib.fileset.toSource {
          root = ./.;

          fileset = pkgs.lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./src
          ];
        };

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          cmake
          pkg-config
        ];

        buildInputs = with pkgs; [
          libgit2
          oniguruma
          openssl
          zlib
        ];

        meta = {
          description = "A minimal terminal frontend for the fff-search library";
          license = pkgs.lib.licenses.mit;

          # The package is named fff-cli, but its executable is bin/fff.
          mainProgram = "fff";
        };
      };
  in {
    packages = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        package = mkPackage pkgs;
      in {
        default = package;
        "fff-cli" = package;
      }
    );

    apps = forAllSystems (
      system: let
        package = self.packages.${system}.default;

        app = {
          type = "app";
          program = "${package}/bin/fff";
        };
      in {
        default = app;
        fff = app;
      }
    );

    overlays.default = final: _previous: {
      fff-cli = mkPackage final;
    };

    devShells = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          inputsFrom = [
            self.packages.${system}.default
          ];

          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
          ];

          RUST_BACKTRACE = "1";
        };
      }
    );
  };
}
