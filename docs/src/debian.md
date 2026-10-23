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
   sudo dpkg -i target/debian/sherlock-launcher_v0.2.4_amd64.deb
   ```
   > You can also use tab-completion to auto complete the file name.

</details>