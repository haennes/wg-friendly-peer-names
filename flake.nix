{
  description = ''
    Provides a way to give wireguard peers a friendlier and more readable name
  '';

  inputs = { nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable"; };

  outputs = { self, nixpkgs }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in {

      packages.x86_64-linux = rec {
        wgg = pkgs.writeShellApplication {
          name = "wgg";
          checkPhase = ":"; # too many failed checks
          bashOptions = [ ]; # unbound variable $1
          runtimeInputs = with pkgs; [ wireguard-tools ];
          text = builtins.readFile ./wgg.sh;
        };
        wggn = pkgs.writeShellApplication {

          name = "wggn";
          checkPhase = ":"; # too many failed checks
          bashOptions = [ ]; # unbound variable $1
          runtimeInputs = with pkgs; [ wireguard-tools ];
          text = ''${wgg}/bin/wgg -n "$@"'';
        };

        default = self.packages.x86_64-linux.wgg;
      };

      nixosModules.default = { config, lib, ... }:
        let
          inherit (lib) concatLines mkEnableOption mapAttrsToList flatten;
          cfg = config.wg-friendly-peer-names;
        in {
          options.wg-friendly-peer-names = {
            enable = mkEnableOption "wg-friendly-peer-names";
            wggn.enable = mkEnableOption "alias for wgg -n";
          };
          config = lib.mkIf (cfg.enable || cfg.wggn.enable) {
            environment.etc."wireguard/peers".text = concatLines 
              (map (peer: "${peer.publicKey}:${peer.name}")
                 (flatten (mapAttrsToList (_: v: v.peers) config.networking.wireguard.interfaces)));
            environment.systemPackages = (lib.optionals (cfg.enable)
              [ self.packages.x86_64-linux.default ])
              ++ (lib.optionals (cfg.wggn.enable))
              [ self.packages.x86_64-linux.wggn ];
          };
        };

      nixosModules.wgg = self.nixosModules.default;
    };
}
