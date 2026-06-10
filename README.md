<div align="center" style="">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
  <img height="250" alt="sherlock logo" src="assets/logo-light.svg">
</picture>

[![Discord](https://img.shields.io/discord/1357746313646833945.svg?color=7289da&&logo=discord)](https://discord.gg/AQ44g4Yp9q)
<img width="100%" alt="application screenshot" src="assets/mockup.png">

</div>

Sherlock is a **fast**, **extensible** application launcher for Wayland, build with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui). Sherlock's widgets inherit from launcher configurations. There are several launcher types, inlclugin a [File Search](), [Emoji Picker](), and [Translator]().

> [!NOTE]
> Sherlock has been rewritten entirely, to be compatible with `GPUI` instead of `GTK4`. This included major refactorings, causing some changes to configuration files.

> [!WARNING]
> Disclaimer: Due to GPUI's development primarily focusing on Zed, some features may not be complete yet. In Sherlock, this is barely noticeable though.

# Getting Started

## Installation

### <ins>Arch Linux</ins>

If you're using Arch Linux, you can install the pre-built binary package with the follwing command:

```bash
yay -S sherlock-launcher-bin
```

Or install the community-maintained `git` build with the following command:

```bash
yay -S sherlock-launcher-git
```

### <ins>Build Debian Package</ins>

To build a `.deb` package directly from source, follow these steps:

Make sure you have the following dependencies installed:

<details>
<summary><strong>Dependencies:</strong></summary>

1. `rust` - [How to install rust](https://www.rust-lang.org/tools/install)
2. `git` - [How to install git](https://github.com/git-guides/install-git)
3. `gtk-4-layer-shell` - [GTK4 Layer Shell](https://github.com/wmww/gtk4-layer-shell)
4. `dbus` - (Used to get currently playing song)

</details>

<details>
<summary><strong>Build Steps:</strong></summary>

1. **Install the** `cargo-deb` **tool:**
   First, you need to install the `cargo-deb` tool, which specifies packaging Rust projects as Debian packages:

   ```bash
   cargo deb
   ```

2. **Build the Debian package**:
   After installing `cargo-deb`, run the following command to build the `.deb` package:

   ```bash
   cargo deb
   ```

3. **Install the generated** `.deb` **package**:
   Once the package is built, you can install it using:

   ```bash
   sudo dpkg -i target/debian/sherlock-launcher_v0.2.3_amd64.deb
   ```
   > You can also use tab-completion to auto complete the file name.

</details>

### <ins>From Source</ins>

To build Sherlock from source, follow these steps. 

Make sure to have the following dependencies installed:

<details>
<summary><strong>Dependencies:</strong></summary>

1. `rust` - [How to install rust](https://www.rust-lang.org/tools/install)
2. `git` - [How to install git](https://github.com/git-guides/install-git)
3. `gtk-4-layer-shell` - [GTK4 Layer Shell](https://github.com/wmww/gtk4-layer-shell)
4. `dbus` - (Used to get currently playing song)

</details>

<details>
<summary><strong>Build Steps:</strong></summary>

1. **Clone the repository**:

   ```bash
   git clone https://github.com/skxxtz/sherlock.git
   cd sherlock
   ```

2. **Build the project using the following command**:

   ```bash
   cargo build --release
   ```

3. **Install the binary**:
   After the build completes, install the binary to your system:

   ```bash
   sudo cp target/release/sherlock /usr/local/bin/
   ```

4. **(Recommended) Remove the build directory**:
   You can optionally remove the source code directory

   ```bash
   rm -rf /path/to/sherlock
   ```

</details>

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

## Post Installation

### Config Setup

After the installation is completed, you can set up your configuration files. Those files live in the `~/.config/sherlock/` directory. Depending on your needs, you should add the following files: 

1. [**config.toml**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/config.toml): This file specifies the behavior and defaults of your launcher. [Documentation](https://github.com/Skxxtz/sherlock/blob/main/docs/config.md)
2. [**fallback.json**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/fallback.json): This file specifies the features your launcher should have. [Documentation](https://github.com/Skxxtz/sherlock/blob/main/docs/launchers.md)
3. [**sherlock_alias.json**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlock_alias.json): This file spcifies aliases for applications. [Documentation](https://github.com/Skxxtz/sherlock/blob/main/docs/aliases.md)
4. [**sherlockignore**](https://github.com/Skxxtz/sherlock/blob/main/docs/examples/sherlockignore): This file specifies applications to exclude from your search. [Documentation](https://github.com/Skxxtz/sherlock/blob/main/docs/sherlockignore.md)

As of `version 0.1.11`, Sherlock comes with the `init` subcommand to automatically create your config. This will create versions of the files above, populated with the default values. Additionally, it will create the `icons/`, `scripts/`, and `themes/` subdirectories. All you have to do is run the following command:

```bash
sherlock init
```

# Contributing

Contributions are welcome! Please follow these guidelines:

**Prerequisites**

Ensure you have the latest stable Rust toolchain installed along with `rustfmt` and `clippy`.

**Branching**

- `main`: stable releases only
- Feature branches: `feat/your-feature`
- Feature branches: `fix/description`

**Before opening a PR**

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

**Releasing**

Releases are automated via `GitHub Actions` on version tags:

```bash
git tag v0.x.0
git push origin v0.x.0
```

# License

GNU GENERAL LICENSE - see [LICENSE](https://github.com/Skxxtz/sherlock/blob/main/LICENSE) for details.