# nix/modules/nixos.nix — auto-generated from lava-forge.caixa.lisp
# description: "Tatara-lisp source generator for lava providers. Consumes terraform providers schema JSON and emits typed (deflava-resource ...) forms. Same upstream that pangea-forge consumes; targets the tatara-lisp surface instead of Ruby."
{ config, lib, pkgs, ... }:
let
  cfg = config.services.lava-forge;
in {
  options.services.lava-forge = {
    enable = lib.mkEnableOption "lava-forge";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.lava-forge or null;
    };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
