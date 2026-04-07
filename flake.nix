{
  description = "Zain - Gestão fiscal via WhatsApp para MEI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    cubos_sql = {
      url = "git+ssh://git@github.com/lbguilherme/cubos_sql.git?ref=main";
      flake = false;
    };
    cubos_sql_cache_agent = { url = "path:./crates/agent/.cubos_sql"; flake = false; };
    cubos_sql_cache_dados_abertos = { url = "path:./crates/dados-abertos/.cubos_sql"; flake = false; };
    cubos_sql_cache_whatsapp = { url = "path:./crates/whatsapp/.cubos_sql"; flake = false; };
  };

  outputs = { self, nixpkgs, crane, cubos_sql, cubos_sql_cache_agent, cubos_sql_cache_dados_abertos, cubos_sql_cache_whatsapp, ... }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    lib = pkgs.lib;
    craneLib = crane.mkLib pkgs;

    src = ./.;

    cargoVendorDir = craneLib.vendorCargoDeps {
      inherit src;
      overrideVendorGitCheckout = ps: drv:
        if lib.any (p:
          lib.hasPrefix "git+ssh://git@github.com/lbguilherme/cubos_sql.git" p.source
        ) ps
        then drv.overrideAttrs (_old: { src = cubos_sql; })
        else drv;
    };

    whatsapp = craneLib.buildPackage {
      pname = "whatsapp";
      inherit src cargoVendorDir;
      cargoExtraArgs = "-p whatsapp";
      LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
      nativeBuildInputs = with pkgs; [ pkg-config clang ];
      buildInputs = with pkgs; [ openssl ];
      postUnpack = ''
        cp -r ${cubos_sql_cache_agent} $sourceRoot/crates/agent/.cubos_sql
        cp -r ${cubos_sql_cache_dados_abertos} $sourceRoot/crates/dados-abertos/.cubos_sql
        cp -r ${cubos_sql_cache_whatsapp} $sourceRoot/crates/whatsapp/.cubos_sql
      '';
    };
  in {
    packages.${system}.whatsapp = whatsapp;

    nixosModules.default = { config, pkgs, lib, ... }: {
      config = {
        users.users.zain = {
          isSystemUser = true;
          group = "zain";
        };
        users.groups.zain = {};

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

          environment = {
            DATABASE_URL = "postgresql:///zain?host=/run/postgresql";
            WEBHOOK_PORT = "3100";
            WHAPI_BASE_URL = "https://gate.whapi.cloud";
            OLLAMA_URL = "http://localhost:11434";
            OLLAMA_MODEL = "gemma4:26b-a4b-it-q4_K_M";
          };

          serviceConfig = {
            ExecStart = "${whatsapp}/bin/whatsapp";
            EnvironmentFile = "/etc/zain.env";
            User = "zain";
            Restart = "always";
            RestartSec = 5;
          };
        };
      };
    };
  };
}
