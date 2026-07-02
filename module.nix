{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.homelab-dashboard;

  # Convert settings to TOML. TOML has no `null`, but many options default to
  # null when unset, so strip null values recursively (including inside lists
  # like `health_checks`) before generating the file.
  settingsFormat = pkgs.formats.toml {};
  stripNulls = v:
    if builtins.isAttrs v
    then lib.filterAttrs (_: x: x != null) (builtins.mapAttrs (_: stripNulls) v)
    else if builtins.isList v
    then map stripNulls v
    else v;
  configFile = settingsFormat.generate "homelab-dashboard.toml" (stripNulls cfg.settings);
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

          search = lib.mkOption {
            type = lib.types.submodule {
              options = {
                type = lib.mkOption {
                  type = lib.types.enum [ "searxng" ];
                  default = "searxng";
                  description = "Search engine the header box submits to.";
                };
                url = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Base URL of the search instance (e.g. SearXNG).";
                };
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

          quick_links = lib.mkOption {
            type = lib.types.listOf (lib.types.submodule {
              options = {
                name = lib.mkOption {
                  type = lib.types.str;
                  description = "Quick link display label.";
                };
                url = lib.mkOption {
                  type = lib.types.str;
                  description = "Destination URL the link points at.";
                };
                icon = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = ''
                    Optional leading icon: either an inline emoji (e.g. "📊")
                    or an http(s) image URL (e.g. an SVG from dashboard-icons).
                  '';
                };
              };
            });
            default = [];
            description = "List of static shortcuts shown in the Quick Links card.";
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
        HOFVARPNIR_API_KEY=xxxxx
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
