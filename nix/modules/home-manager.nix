# nix/modules/home-manager.nix — auto-generated from lava-forge.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.programs.lava-forge; in {
  options.programs.lava-forge = {
    enable = lib.mkEnableOption "lava-forge";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-forge or null; };
  };
  config = lib.mkIf cfg.enable { home.packages = [ cfg.package ]; };
}
