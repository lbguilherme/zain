{
  description = "Zain - Gestão fiscal via WhatsApp para MEI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    cubos_sql = {
      url = "git+ssh://git@github.com/lbguilherme/cubos_sql.git?ref=main";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, cubos_sql, ... }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    lib = pkgs.lib;
    craneLib = crane.mkLib pkgs;

    src = craneLib.cleanCargoSource ./.;

    cargoVendorDir = craneLib.vendorCargoDeps {
      inherit src;
      overrideVendorGitCheckout = ps: drv:
        if lib.any (p:
          lib.hasPrefix "git+ssh://git@github.com/lbguilherme/cubos_sql.git" p.source
        ) ps
        then drv.overrideAttrs (_old: { src = cubos_sql; })
        else drv;
    };

    envFile = builtins.readFile ./.env;
    parseEnv = text: lib.listToAttrs (lib.concatMap (line:
      let m = builtins.match "([A-Za-z_][A-Za-z0-9_]*)=(.*)" line;
      in if m != null then [{ name = builtins.elemAt m 0; value = builtins.elemAt m 1; }] else []
    ) (lib.splitString "\n" text));
    env = parseEnv envFile;

    whatsapp = craneLib.buildPackage {
      pname = "whatsapp";
      inherit src cargoVendorDir;
      cargoExtraArgs = "-p whatsapp";
      nativeBuildInputs = with pkgs; [ pkg-config ];
      buildInputs = with pkgs; [ openssl ];
    };
  in {
    packages.${system}.whatsapp = whatsapp;

    nixosModules.default = { config, pkgs, lib, ... }: {
      config = {
        services.postgresql = {
          ensureDatabases = [ "zain" ];
          ensureUsers = [
            {
              name = "zain";
              ensureDBOwnership = true;
            }
          ];
        };

        systemd.services.zain-whatsapp = {
          description = "Zain WhatsApp webhook + outbox";
          after = [ "network.target" "postgresql.service" ];
          wants = [ "postgresql.service" ];
          wantedBy = [ "multi-user.target" ];

          environment = env // {
            DATABASE_URL = "postgresql:///zain?host=/run/postgresql";
          };

          serviceConfig = {
            ExecStart = "${whatsapp}/bin/whatsapp";
            User = "zain";
            Restart = "always";
            RestartSec = 5;
          };
        };
      };
    };
  };
}
