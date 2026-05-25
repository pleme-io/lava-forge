# nix/modules/darwin.nix — auto-generated from lava-forge.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.services.lava-forge; in {
  options.services.lava-forge = {
    enable = lib.mkEnableOption "lava-forge";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-forge or null; };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
