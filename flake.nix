{
  description = "PJtei - Contabilidade via WhatsApp para MEI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }: {
    nixosModules.default = { config, pkgs, lib, ... }: {
      services.postgresql = {
        ensureDatabases = [ "pjtei" ];
        ensureUsers = [
          {
            name = "pjtei";
            ensureDBOwnership = true;
          }
        ];
      };
    };
  };
}
