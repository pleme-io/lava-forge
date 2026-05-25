# flake.nix — auto-generated from lava-forge.caixa.lisp
# Edit caixa source + re-render via:
#   pleme-doc-gen caixa --source lava-forge.caixa.lisp --out . --force
{
  description = "lava-forge — caixa-rendered Nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ { self, nixpkgs, substrate, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "aarch64-darwin" "x86_64-linux" "aarch64-linux" ];
      perSystem = { pkgs, system, ... }: let
        builder = substrate.lib.${system}.rustToolReleaseFlakeBuilder;
      in {
        packages.default = builder {
          inherit pkgs;
          name = "lava-forge";
          src = ./.;
        };
        devShells.default = pkgs.mkShell {
          buildInputs = [ pkgs.git ];
        };
      };
    };
}
