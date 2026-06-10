use indoc::indoc;
use md_rs::{
    components::{
        ParentComponentExt, TextComponentExt,
        code_block::codeblock,
        container::Container,
        details::{Details, details},
        heading::{h2, h3, h4},
        span::{bold, br, code, html_strong, html_underline, link, link_bold},
        span_nodes::blockquote,
    },
    item, list, list_iter, md, p,
};

use crate::docs::{
    Documentation,
    book::{BookEntry, TopLevelEntry},
};

pub(super) struct Installation;

impl Documentation for Installation {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h2("Installation"),
            ArchLinux::docs(),
            Debian::docs(),
            Source::docs(),
            NixOS::docs(),
            PostInstallation::docs(),
        )
    }
}

impl TopLevelEntry for Installation {
    type Summary = Container;
    fn summary() -> Self::Summary {
        md!(
            h2(html_underline("Installation")),
            p!(
                "Sherlock can be installed on a variety of Linux distributions.",
                br(),
                "Choose your distribution below to get started:"
            ),
            list_iter!(
                Dash,
                Self::children().map(|child| link(child.title, child.file.unwrap_or("#"))),
            )
        )
    }
    fn children() -> impl Iterator<Item = BookEntry> + 'static {
        [
            BookEntry::of::<ArchLinux>()
                .with_title("Arch Linux")
                .with_file("arch-linux.md"),
            BookEntry::of::<Debian>()
                .with_title("Debian / Ubuntu")
                .with_file("debian.md"),
            BookEntry::of::<NixOS>()
                .with_title("NixOs")
                .with_file("nixos.md"),
            BookEntry::of::<Source>()
                .with_title("Build from Source")
                .with_file("source.md"),
        ]
        .into_iter()
    }
}

impl From<Installation> for BookEntry {
    fn from(_: Installation) -> Self {
        BookEntry {
            title: "Installation",
            file: Some("installation.md"),
            render_fn: Some(Installation::summary_md),
            children: Installation::children().collect(),
        }
    }
}

struct ArchLinux;
impl Documentation for ArchLinux {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h3(html_underline("Arch Linux")),
            "If you're using Arch Linux, you can install the pre-built \
            binary package with the follwing command:",
            codeblock()
                .lang("bash")
                .content("yay -S sherlock-launcher-bin"),
            p!(
                "Or install the community-maintained",
                code("git"),
                "build with the following command:"
            ),
            codeblock()
                .lang("bash")
                .content("yay -S sherlock-launcher-git"),
        )
    }
}

struct Source;
impl Documentation for Source {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h3(html_underline("From Source")),
            p!(
                "To build Sherlock from source, follow these steps.",
                br(),
                "Make sure to have the following dependencies installed:"
            ),
            BuildDependencies::docs(),
            details().summary(html_strong("Build Steps:")).child(list!(
                Ordered,
                item!(
                    p!(bold("Clone the repository"), ":"),
                    codeblock()
                        .lang("bash")
                        .line("git clone https://github.com/skxxtz/sherlock.git")
                        .line("cd sherlock"),
                ),
                item!(
                    p!(bold("Build the project using the following command"), ":"),
                    codeblock().lang("bash").line("cargo build --release"),
                ),
                item!(
                    p!(bold("Install the binary"), ":"),
                    "After the build completes, install the binary to your system:",
                    codeblock()
                        .lang("bash")
                        .line("sudo cp target/release/sherlock /usr/local/bin/"),
                ),
                item!(
                    p!(bold("(Recommended) Remove the build directory"), ":"),
                    "You can optionally remove the source code directory",
                    codeblock().lang("bash").line("rm -rf /path/to/sherlock"),
                ),
            )),
        )
    }
}

struct NixOS;
impl Documentation for NixOS {
    type Docs = Container;
    fn docs() -> Self::Docs {
        let non_flake_systems = md!(
            h4("Non-Flake Systems"),
            p!(
                "Sherlock is available in",
                code("nixpkgs/unstable"),
                "as",
                code("sherlock-launcher"),
                ". If you're installing it as a standalone package, \
                you'll need to do the",
                link("config setup", "#config-setup"),
                "yourself."
            ),
        );

        let flakes_with_home_manager = md!(
            h4("Flakes & Home-Manager"),
            p!(
                "A module for Sherlock is available in home manager. \
                You can find it's configuration",
                link(
                    "here",
                    "https://github.com/nix-community/home-manager/blob\
                        /master/modules/programs/sherlock.nix",
                ),
                ". If you want to use the lastest updates and module options, \
                follow the steps below.",
            ),
            details()
                .summary(html_strong("Home-Manager Example Configuration"))
                .child(p!(
                    "Add the floowing",
                    code("inputs"),
                    "of",
                    code("flake.nix"),
                    "if you want to use the lastest upstream version of Sherlock."
                ),)
                .child(codeblock().lang("nix").content(indoc! {r#"
                    sherlock = {
                        url = "github:Skxxtz/sherlock";
                        inputs.nixpkgs.follows = "nixpkgs";
                    };
                "#}))
                .child("Home-Manager config:")
                .child(codeblock().lang("nix").content(indoc! {r#"
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
                    "#})),
        );

        let flakes_without_home_manager = md!(
            h4("Flakes without Home-Manager"),
            p!(
                "To install the standalone package, add",
                code("sherlock.packages.${pkgs.system}.default"),
                "to",
                code("environment.systemPackages"),
                "/",
                code("home.packages"),
                ". You will need to create the configuration files yourself, see below.",
            )
        );

        let cachix = md!(
            h4("Cachix"),
            p!(
                "To use pre-built binaries, use:",
                code("cachix use sherlock"),
                br(),
                "or:"
            ),
            codeblock().lang("nix").content(indoc!{r#"
                    nix.settings = {
                      substituters = ["https://sherlock.cachix.org"];
                      trusted-public-keys = ["sherlock.cachix.org-1:w6O/gUQB2CRFXKg7NfAAR+FGtotlj0tUi3dscRUKpX0="];
                    }
                "#})
        );

        md!(
            h3(html_underline("NixOS")),
            cachix,
            non_flake_systems,
            flakes_with_home_manager,
            flakes_without_home_manager,
        )
    }
}

struct Debian;
impl Documentation for Debian {
    type Docs = Container;
    fn docs() -> Self::Docs {
        let build_step_1 = item!(
            p!(bold("Install the"), code("cargo-deb"), bold("tool:")),
            p!(
                "First, you need to install the",
                code("cargo-deb"),
                "tool, which specifies packaging Rust projects as Debian packages:"
            ),
            codeblock().lang("bash").line("cargo deb")
        );

        let build_step_2 = item!(
            p!(bold("Build the Debian package"), ":"),
            p!(
                "After installing",
                code("cargo-deb"),
                ", run the following command to build the",
                code(".deb"),
                "package:"
            ),
            codeblock().lang("bash").line("cargo deb")
        );

        let build_step_3 = item!(
            p!(
                bold("Install the generated"),
                code(".deb"),
                bold("package"),
                ":"
            ),
            "Once the package is built, you can install it using:",
            codeblock().lang("bash").line(concat!(
                "sudo dpkg -i target/debian/sherlock-launcher_v",
                env!("CARGO_PKG_VERSION"),
                "_amd64.deb"
            )),
            blockquote().text("You can also use tab-completion to auto complete the file name."),
        );

        md!(
            h3(html_underline("Build Debian Package")),
            p!(
                "To build a",
                code(".deb"),
                "package directly from source, follow these steps:"
            ),
            "Make sure you have the following dependencies installed:",
            BuildDependencies::docs(),
            details().summary(html_strong("Build Steps:")).child(list!(
                Ordered,
                build_step_1,
                build_step_2,
                build_step_3,
            )),
        )
    }
}

struct BuildDependencies;
impl Documentation for BuildDependencies {
    type Docs = Details;
    fn docs() -> Self::Docs {
        details().summary(html_strong("Dependencies:")).child(list!(
            Ordered,
            p!(
                code("rust"),
                "-",
                link(
                    "How to install rust",
                    "https://www.rust-lang.org/tools/install",
                )
            ),
            p!(
                code("git"),
                "-",
                link(
                    "How to install git",
                    "https://github.com/git-guides/install-git",
                )
            ),
            p!(
                code("gtk-4-layer-shell"),
                "-",
                link(
                    "GTK4 Layer Shell",
                    "https://github.com/wmww/gtk4-layer-shell",
                )
            ),
            p!(code("dbus"), "-", "(Used to get currently playing song)"),
        ))
    }
}

struct PostInstallation;
impl Documentation for PostInstallation {
    type Docs = Container;
    fn docs() -> Self::Docs {
        let config_files = list!(
            Ordered,
            p!(
                link_bold(
                    "config.toml",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/examples/config.toml",
                ),
                ": This file specifies the behavior and defaults of your launcher.",
                link(
                    "Documentation",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/config.md",
                ),
            ),
            p!(
                link_bold(
                    "fallback.json",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/examples/fallback.json",
                ),
                ": This file specifies the features your launcher should have.",
                link(
                    "Documentation",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/launchers.md",
                )
            ),
            p!(
                link_bold(
                    "sherlock_alias.json",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/examples/sherlock_alias.json",
                ),
                ": This file spcifies aliases for applications.",
                link(
                    "Documentation",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/aliases.md",
                ),
            ),
            p!(
                link_bold(
                    "sherlockignore",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/examples/sherlockignore",
                ),
                ": This file specifies applications to exclude from your search.",
                link(
                    "Documentation",
                    "https://github.com/Skxxtz/sherlock/blob\
                    /main/docs/sherlockignore.md",
                ),
            ),
        );

        md!(
            h2("Post Installation"),
            h3("Config Setup"),
            p!(
                "After the installation is completed, \
                you can set up your configuration files. \
                Those files live in the",
                code("~/.config/sherlock/"),
                "directory. Depending on your needs, \
                you should add the following files: ",
            ),
            config_files,
            p!(
                "As of",
                code("version 0.1.11"),
                ", Sherlock comes with the",
                code("init"),
                "subcommand to automatically create your config. This \
                will create versions of the files above, \
                populated with the default values. Additionally, \
                it will create the",
                code("icons/"),
                ",",
                code("scripts/"),
                ", and",
                code("themes/"),
                "subdirectories. All you have to do is run the following command:",
            ),
            codeblock().lang("bash").line("sherlock init"),
        )
    }
}
