{
  description = ''
    Provides a way to give wireguard peers a friendlier and more readable name
  '';

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ (import rust-overlay) ];
      };
      rev = self.shortRev or self.dirtyShortRev or "dirty";
      wgg = pkgs.callPackage ./package.nix { inherit rev; };
    in
    {

      packages.x86_64-linux = {
        inherit wgg;

        default = self.packages.x86_64-linux.wgg;
      };
      devShells.x86_64-linux = rec {
        default = completions;
        completions = pkgs.mkShell rec {
          strictDeps = true;
          nativeBuildInputs = [ wgg ];
          buildInputs = nativeBuildInputs;
        };
      };
      formatter.x86_64-linux = pkgs.nixfmt-rfc-style;

      nixosModules.default =
        { config, lib, ... }:
        let
          inherit (lib)
            concatLines
            mkEnableOption
            mapAttrsToList
            flatten
            ;
          cfg = config.wg-friendly-peer-names;
        in
        {
          options.wg-friendly-peer-names = {
            enable = mkEnableOption "wg-friendly-peer-names";
            enableSUID = mkEnableOption "wether to enable registering this binary as suid sou you can run it wihout sudo";
          };
          config = lib.mkIf cfg.enable {
            environment.etc."wireguard/peers".text = concatLines (
              map (peer: "${peer.publicKey}:${peer.name}") (
                flatten (mapAttrsToList (_: v: v.peers) config.networking.wireguard.interfaces)
              )
            );
            environment.systemPackages = [ self.packages.x86_64-linux.default ];
            security.wrappers = lib.mkIf cfg.enableSUID {
              "wgg" = {
                setgid = true;
                setuid = true;
                owner = "root";
                group = "root";
                source = lib.getExe wgg;
              };
            };
          };
        };

      nixosModules.wgg = self.nixosModules.default;
    };
}
