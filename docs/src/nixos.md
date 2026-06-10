### <ins>NixOS</ins>

#### Cachix

To use pre-built binaries, use: `cachix use sherlock` 

or:

```nix
nix.settings = {
  substituters = ["https://sherlock.cachix.org"];
  trusted-public-keys = ["sherlock.cachix.org-1:w6O/gUQB2CRFXKg7NfAAR+FGtotlj0tUi3dscRUKpX0="];
}
```

#### Non-Flake Systems

Sherlock is available in `nixpkgs/unstable` as `sherlock-launcher`. If you're installing it as a standalone package, you'll need to do the [config setup](#config-setup) yourself.

#### Flakes & Home-Manager

A module for Sherlock is available in home manager. You can find it's configuration [here](https://github.com/nix-community/home-manager/blob/master/modules/programs/sherlock.nix). If you want to use the lastest updates and module options, follow the steps below.

<details>
<summary><strong>Home-Manager Example Configuration</strong></summary>

Add the floowing `inputs` of `flake.nix` if you want to use the lastest upstream version of Sherlock.

```nix
sherlock = {
    url = "github:Skxxtz/sherlock";
    inputs.nixpkgs.follows = "nixpkgs";
};
```

Home-Manager config:

```nix
programs.sherlock = {
    enable = true;

    # to run sherlock as a daemon
    systemd.enable = true;

    # If wanted, you can use this line for the _latest_ package.
    # Otherwise, you're relying on nixpkgs to update it frequently enough.
    # For this to work, make sure to add sherlock as a flake input!
    # package = inputs.sherlock.packages.${pkgs.system}.default;

    # config.toml
    settings = {};

    # sherlock_alias.json
    aliases = {
        vesktop = { name = "Discord"; };
    };

    # sherlockignore
    ignore = ''
        Avahi*
    '';

    # fallback.json
    launchers = [
        {
            name = "Calculator";
            type = "calculation";
            args = {
                capabilities = [
                    "calc.math"
                    "calc.units"
                ];
            };
            priority = 1;
        }
        {
            name = "App Launcher";
            type = "apps";
            args = {};
            priority = 2;
            home = "Home";
        }
    ];
};
```

</details>

#### Flakes without Home-Manager

To install the standalone package, add `sherlock.packages.${pkgs.system}.default` to `environment.systemPackages` / `home.packages`. You will need to create the configuration files yourself, see below.