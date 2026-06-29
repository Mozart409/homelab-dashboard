{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.homelab-dashboard;

  # Convert settings to TOML
  settingsFormat = pkgs.formats.toml {};
  configFile = settingsFormat.generate "homelab-dashboard.toml" cfg.settings;
in {
  options.services.homelab-dashboard = {
    enable = lib.mkEnableOption "Homelab Dashboard";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.homelab-dashboard or (throw "homelab-dashboard package not found. Add the overlay to your nixpkgs.");
      description = "The homelab-dashboard package to use.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "dashboard";
      description = "User under which the dashboard runs.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "dashboard";
      description = "Group under which the dashboard runs.";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/homelab-dashboard";
      description = "Directory for dashboard data.";
    };

    settings = lib.mkOption {
      type = lib.types.submodule {
        freeformType = settingsFormat.type;

        options = {
          listen_address = lib.mkOption {
            type = lib.types.str;
            default = "127.0.0.1";
            description = "Address to listen on.";
          };

          port = lib.mkOption {
            type = lib.types.port;
            default = 8080;
            description = "Port to listen on.";
          };

          searxng = lib.mkOption {
            type = lib.types.submodule {
              options.url = lib.mkOption {
                type = lib.types.nullOr lib.types.str;
                default = null;
                description = "SearXNG instance URL.";
              };
            };
            default = {};
          };

          weather = lib.mkOption {
            type = lib.types.submodule {
              options = {
                latitude = lib.mkOption {
                  type = lib.types.float;
                  default = 52.52;
                  description = "Weather location latitude.";
                };
                longitude = lib.mkOption {
                  type = lib.types.float;
                  default = 13.41;
                  description = "Weather location longitude.";
                };
                location = lib.mkOption {
                  type = lib.types.str;
                  default = "Berlin";
                  description = "Weather location name.";
                };
              };
            };
            default = {};
          };

          proxmox = lib.mkOption {
            type = lib.types.submodule {
              options = {
                url = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Proxmox VE URL.";
                };
                token_id = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Proxmox API token ID (user@realm!tokenid).";
                };
                token_secret = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Proxmox API token secret. Consider using secretFile instead.";
                };
              };
            };
            default = {};
          };

          jellyfin = lib.mkOption {
            type = lib.types.submodule {
              options = {
                url = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Jellyfin server URL.";
                };
                api_key = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Jellyfin API key. Consider using secretFile instead.";
                };
              };
            };
            default = {};
          };

          homeassistant = lib.mkOption {
            type = lib.types.submodule {
              options = {
                url = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Home Assistant URL.";
                };
                token = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Home Assistant long-lived access token. Consider using secretFile instead.";
                };
                entity_ids = lib.mkOption {
                  type = lib.types.listOf lib.types.str;
                  default = [];
                  description = "Home Assistant entity IDs to display (empty shows all).";
                };
              };
            };
            default = {};
          };

          hofvarpnir = lib.mkOption {
            type = lib.types.submodule {
              options = {
                url = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Hofvarpnir URL.";
                };
                api_key = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Hofvarpnir API key.";
                };
              };
            };
            default = {};
          };

          health_checks = lib.mkOption {
            type = lib.types.listOf (lib.types.submodule {
              options = {
                name = lib.mkOption {
                  type = lib.types.str;
                  description = "Health check display name.";
                };
                url = lib.mkOption {
                  type = lib.types.str;
                  description = "URL to check.";
                };
                timeout_ms = lib.mkOption {
                  type = lib.types.int;
                  default = 5000;
                  description = "Request timeout in milliseconds.";
                };
                expected_status = lib.mkOption {
                  type = lib.types.nullOr lib.types.int;
                  default = null;
                  description = "Expected HTTP status code (default: 200).";
                };
              };
            });
            default = [];
            description = "List of health check endpoints.";
          };
        };
      };
      default = {};
      description = "Dashboard configuration.";
    };

    secretsFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Path to a file containing secrets as environment variables.
        This file should contain lines like:
        PROXMOX_TOKEN_SECRET=xxxxx
        JELLYFIN_API_KEY=xxxxx
        HOMEASSISTANT_TOKEN=xxxxx
      '';
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to open the firewall for the dashboard port.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Create user and group
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = cfg.dataDir;
      createHome = true;
    };

    users.groups.${cfg.group} = {};

    # Systemd service
    systemd.services.homelab-dashboard = {
      description = "Homelab Dashboard";
      wantedBy = ["multi-user.target"];
      after = ["network.target"];

      environment = {
        RUST_LOG = "info,dashboard_server=debug,dashboard_app=debug";
        DASHBOARD_STATIC_DIR = "${cfg.package}/share/dashboard";
      };

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.dataDir;

        ExecStart = "${cfg.package}/bin/dashboard-server";

        # Load config file
        ExecStartPre = [
          "${pkgs.coreutils}/bin/install -m 600 ${configFile} ${cfg.dataDir}/config.toml"
        ];

        # Load secrets if provided
        EnvironmentFile = lib.mkIf (cfg.secretsFile != null) cfg.secretsFile;

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = ["AF_INET" "AF_INET6" "AF_UNIX"];
        RestrictNamespaces = true;
        LockPersonality = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;

        ReadWritePaths = [cfg.dataDir];

        # Restart on failure
        Restart = "on-failure";
        RestartSec = "5s";
      };
    };

    # Firewall
    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [cfg.settings.port];
  };
}
