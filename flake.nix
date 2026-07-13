{
  description = "Development shell for fff-cli";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = function:
      builtins.listToAttrs (map (system: {
          name = system;
          value = function system;
        })
        systems);
  in {
    devShells = forAllSystems (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          cmake
          libgit2
          oniguruma
          openssl
          pkg-config
          rustc
          rustfmt
          zlib
        ];

        RUST_BACKTRACE = "1";
      };
    });
  };
}
