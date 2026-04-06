{
  description = "Zain - Gestão fiscal via WhatsApp para MEI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }: {
    nixosModules.default = { config, pkgs, lib, ... }: {
      services.postgresql = {
        ensureDatabases = [ "zain" ];
        ensureUsers = [
          {
            name = "zain";
            ensureDBOwnership = true;
          }
        ];
      };
    };
  };
}
