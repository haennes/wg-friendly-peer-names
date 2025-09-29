{
  description = ''
    Provides a way to give wireguard peers a friendlier and more readable name
  '';

  inputs = { nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable"; };

  outputs = { self, nixpkgs }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux;
        wgg = pkgs.stdenv.mkDerivation {
          name="wireguard-friendly-peer-names";
          pname = "wgg";
          checkPhase = ":"; # too many failed checks
          src = ./.;

          nativeBuildInputs = with pkgs; [ makeWrapper ];
          installPhase = ''
              mkdir -p $out/bin
              cp wgg.sh $out/bin/wgg
              wrapProgram $out/bin/wgg --prefix PATH : ${pkgs.wireguard-tools}/bin
            '';
        };
        wggnf = preCmd: pkgs.writeShellApplication {

          name = "wggn";
          checkPhase = ":"; # too many failed checks
          bashOptions = [ ]; # unbound variable $1
          runtimeInputs = with pkgs; [ wireguard-tools ];
          text = ''${preCmd} ${wgg}/bin/wgg -n'';
        };
        wggn = wggnf "";
    in {

      packages.x86_64-linux = {
        inherit wgg wggn;

        default = self.packages.x86_64-linux.wgg;
      };

      nixosModules.default = { config, lib, ... }:
        let
          inherit (lib) concatLines mkEnableOption mapAttrsToList flatten;
          cfg = config.wg-friendly-peer-names;
        in {
          options.wg-friendly-peer-names = {
            enable = mkEnableOption "wg-friendly-peer-names";
            wggn = {
              enable = mkEnableOption "alias for wgg -n";
              enableSUID = mkEnableOption "wether to enable registering this binary as suid sou you can run it wihout sudo";
            };
          };
          config = lib.mkIf (cfg.enable || cfg.wggn.enable) {
            environment.etc."wireguard/peers".text = concatLines 
              (map (peer: "${peer.publicKey}:${peer.name}")
                 (flatten (mapAttrsToList (_: v: v.peers) config.networking.wireguard.interfaces)));
            environment.systemPackages = (lib.optionals (cfg.enable)
              [ self.packages.x86_64-linux.default ])
              ++ (lib.optionals (cfg.wggn.enable))
              [ self.packages.x86_64-linux.wggn ];
            security.wrappers = lib.mkIf cfg.wggn.enableSUID {
              "wggn" = {
                setgid = true;
                setuid = true;
                owner = "root";
                group = "root";
                source = "${wggn}/bin/wggn";
                # source = "${wggnf "sudo"}/bin/wggn";
              };
            };
          };
        };

      nixosModules.wgg = self.nixosModules.default;
    };
}
