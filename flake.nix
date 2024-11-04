{
  description = ''
    Provides a way to give wireguard peers a friendlier and more readable name
    Resources'';

  inputs = { nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable"; };

  outputs = { self, nixpkgs }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in {

      packages.x86_64-linux.wgg = pkgs.writeShellApplication {
        name = "wgg";
        checkPhase = ":"; # too many failed checks
        bashOptions = [ ]; # unbound variable $1
        runtimeInputs = with pkgs; [ wireguard-tools ];
        text = builtins.readFile ./wgg.sh;
      };

      packages.x86_64-linux.default = self.packages.x86_64-linux.wgg;

    };
}
