# Changelog

All notable changes to the **Sherlock Launcher** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

## Changelog Categories

### Changelog Category Definitions

* `Added`: Use for new features or capabilities that did not exist previously.
* `Changed`: Use for modifications to existing functionality or API updates.
* `Deprecated`: Use for features that are still present but scheduled for
  removal in future versions.
* `Removed`: Use for features that have been deleted or disabled.
* `Fixed`: Use for any bug fixes, unintended behavior repairs, or logic
  corrections.
* `Security`: Use specifically for patches addressing vulnerabilities or
  security exploits.

---

### Industry Common Additions

* `Improved`: Use for performance optimizations or refinements to existing features.
* `Performance`: Use for significant speed or memory usage improvements.

## Versioning Scheme

1. MAJOR version when you make incompatible API changes
2. MINOR version when you add functionality in a backward compatible manner
3. PATCH version when you make backward compatible bug fixes

Additional labels for pre-release and build metadata are available as
extensions to the MAJOR.MINOR.PATCH format.

run `git log main..dev` for all changes

---

## [Unreleased]

* as of **22a08f87f26b6b0c9cac4277387b8f70a1a9d83e**

## [0.2.3-dev] - 03.06.26

### Added

* **Backdrop:** Added backdrop, including new `animation_duration` key. If left
  empty, will not animate. (`e2393b2e`)
* **MD-RS:** Added badge, image, and div components (`051f45e2`, `051f45e2`,
  `eda442fd`, `fedf0a56`)
* **Search Results:** Added limiting functionality to restrict the number of
  search results displayed per launcher (`e963bacb`)
* **Command Completion:** Added support for fetching executable files located
  outside the standard `$PATH` (`a9071661`)
* **Search Bar:** Enhanced interface with native PNG/symbolic asset icons, a
  blinking cursor animation, and a selected field indicator (`31a3c98b`,
  `4de3e585`, `45e9edc7`, `85e499b2`)
* **Variable Input:** Added choice dropdown fields to variable input selections
  (`52185ef7`)
* **Documentation:** Added automatic documentation generation utilities for
  launchers and integrated them into the release pipeline (`68194737`,
  `e330f121`, `c28b6bdd`)
* **Subcommand:** Added a `sherlock repair` subcommand to fix corrupted
  `fallback.json` and counter files (`0dbe376f`)
* **Config Watcher:** Added a dynamic file watcher to notify the system when
  application configurations change (`c1c77b8f`)
* **User Actions:** Added mouse click support to focus/execute items and
  introduced in-place context action executions (`84ee0cff`, `7575d0ce`)
* **Debug Utility:** Added a built-in debug launcher utility and integrated
  timer start capabilities from the Sherlock CLI (`3fde4978`, `bd0c766b`)

### improvements

* **Flags:** Added `clear_cache` flag to remove cache. (`b0761f84`)
* **Icons:** Added new fallback icon for search icon (`sherlock-search`). (`ac2024a5`)
* **Config Watcher:** Config watcher now also alerts on cache changes. (`1a989d87`)
* **Variables:** Runtime variables can now be nested. (`13632119`)
* **Warnings & Errors:** Improved UI design for warnings and errors. (`de88aa04`)
* **NixOS:**
  * Added `cachix`. (`f6041508`)
  * Update hashes. (`20d00580`)
* **Context Menu Actions:** Limit context menu width to 250px. (`889054f6`)
* **Documentation:** Added README.md and CONTRIBUTING.md auto-gen. (`9b639fde`, `d5bfeccd`)
* **MD-RS Crate:** Improved formatting of md-rs create for documentation auto-generation and
  refactors. (`0a83ea14`, `5bbfa568`, `7a8b0acb`, `36df4464`, `d5bfeccd`)
* **Changelog:** Configured changelog generation to completely ignore
  unreleased changes (`a5bb592f`)
* **Client-Server:** Added a server-side `FIN` window-close message, a
  `-w/--wait` flag to pause for output, and direct piping capabilities
  (`628c4f50`, `74321634`)
* **Pango Parser:** Optimized rendering performance, added benchmarking suites,
  and streamlined span state transitions (`b7b61fc0`, `f0b04a3f`)
* **Timer:** Rewrote system timers to persist and continue running through
  system sleep mode (`e22f5a64`)
* **Architecture:** Shifted heavy `RenderableChild` clones to asynchronous
  context tasks and refactored core layout modules (`b38473b9`, `c617fbd9`,
  `6f16db3b`)
* **Binds & Parsing:** Enhanced keybindings to accept outer functions and
  refactored launcher parsing to map the `name` key directly (`73eed652`,
  `3e34cbfc`, `6c01dfc7`)
* **Core Systems:** Improved URL detection accuracy, application loading
  sequences, error notation layout, and counter tracking (`f167cceb`,
  `27e0c276`, `18e9fdff`, `dea10500`)

### Removed

* **Flags:** Removed `sherlock docs` flag – only ever used in debug. (`c60bd4f1`)
* **Cargo Config:** Removed the default target entry from the internal Cargo
  `config.toml` setup (`60a4b49f`)
* **Icon Cache:** Deleted the obsolete `clear icon cache` utility function
  (`238414d2`)

### Fixed

* **Theme:** Fixed incorrect path to be used in gtk-theme parsing. (`4ff02c72`)
* **Search Icon:** Fixed symbolic search icons not receiving correct color override. (`95064762`)
* **Hot-Loading:**
  * NixOs: Fixed hot-loading of new applications not working on NixOS.
      (`9e23d618`)
  * Alias: Fixed alias hot-loading. (`3a3b5c9e`)
  * Ignore: Fixed sherlockignore hot-loading. (`901c9e88`)
* **Startup:**
  * Fixed long startup delay after first Sherlock client call. (`c99f927e`)
  * Removed flicker due to async filtering, especially noticeable on
      `Backdrop=enabled`. (`22a08f87`)
* **Context Menu Actions:**
  * Fixed `ContextMenuActions` using `get_content` even if valid `exec` is
      provided. (`842da39d`)
  * Fixed launcher view not updating when context menus update (for example
      in script launcher when it supplies new context menu actions).
      (`842da39d`)
* **Release Bot:** Release flags were only running when there was an active
  Sherlock server instance running. Now also works with no server running.
  (`dba461ae`, `83212d78`)
* **Variable Input:** Fixed fields to correctly reset focus back to the search
  bar immediately following execution (`2d352980`)
* **Icons & Assets:** Resolved rendering, asset caching, and color assignment
  bugs for SVGs and clipboard icons (`7b9fcb8f`, `913ddd87`, `57a2e255`)
* **Navigation Focus:** Patched text cursor blinking animations and prevented
  focus or alias contexts from leaking into hidden layout views (`660f9ea4`,
  `23af41e5`, `89f7ec0d`)
* **Documentation:** Fixed layout formatting errors, typographical mistakes,
  and unhandled details blocks (`3467591b`, `23be3dd4`)
* **App Loading:** Corrected context menu parsing failures and resolved an
  inverted search scoring order glitch (`8f877c24`, `949e28eb`, `19fe55c0`)
* **Execution Glitches:** Ensured launchers without actions handle returns
  smoothly and guarded against crashes from oversized intent payloads
  (`9b080570`, `cb668086`, `35c93477`)
* **Media Layout:** Fixed the MPRIS media view to safely hide the lower layout
  column if no artist metadata tags exist (`862b140f`)

## [0.2.2-dev] - 28.04.26

### Added

* **Timer:** Added timer launcher with up to 4 simultaneous timers
* **Variable Input:** Added command variable input with auto-completion
* **Process Launcher:** Added process launcher
* **Theme Picker:** Added theme picker

### improvements

* **Errors/Warnings:** Improved design for `info` tiles
* **Intents:** Improved intent parsing and fixed errors

### Fixed

* **Sherlock init:** Fixed sherlock init configs and removed auto-gen of `fallback.json`
* **Application Actions:** Resolved an issue where Application actions would
  only be applied to the application if an alias for that file existed.

## [0.2.1-dev] - 26.04.26

### Added

* **Config:** Implemented default `fallback.json` file if none is provided by
  the user
* **User Actions:** Added functionality to the clipboard launcher to open URLs
  based on intent
* **Piped Input:** Added basic support for piped input using *dmenu-style*
  newline splitting
* **Sub-menu Flag:** Implemented `-sm` / `--sub-menu` flag functionality and
  added smoother transitions
* **Alias Execution** Added automatic alias execution for file search and emoji
  picker

### Improvements

* **Animations:** Added ease animation for the weather launcher
* **NL Intents:** Improved intent parsing to use iterators instead of
  `smallvecs`
* **Currencies:** Improved currency factor fetching (extensibility,
  cleanliness)
* **Chore:** Refactored multiple code sections according to `clippy`
  suggestions

### Fixed

* **Weather:** Fixed `wttr.in` repeating format change
* **Networking:** Resolved a `tokio::net::UnixSocket` issue that limited the
  application to 64 concurrent openings
* **Launcher Parsing:** Fixed general launcher parsing issues
* **Tests:** Removed redundant clipboard watcher test causing issues in
  automated builds

## [0.2.0-dev] - 20.04.26

### Added

* **Translator:** Added new translation functionality.
* **Launchers:**
  * Implemented **Script Launcher** with "wait for return" support.
  * Implemented **Events Launcher** with `look_ahead` and `look_back` parameter parsing.
  * Added **Web Launcher** application action type.
* **Emoji Picker:** Added foundational picker, including context menus and default skin tone support.
* **File Search:** Implemented basic file search supporting `ripgrep` and `walkdir` backends.
* **Clipboard:** Added clipboard listener and functionality.
* **Integration:** Added support for Zoom meetings and XDG-settings mimetype handling.

### Improvements

* **UI/UX:**
  * Added visual shortcuts.
  * Improved design for script, event, and file tiles.
  * Added animations and improved transition handling.
  * Improved error views (made scrollable, cleaner design).
  * Improved icon theme loading and SVG rendering.
* **Performance & Async:**
  * Decoupled async updates; tiles update as soon as they are ready.
  * Refactored search scoring and improved fuzzy matching.
* **Configuration:**
  * Added configuration hot-reloading (reloads when files change).
  * Improved error handling for currency exchange rate updates and empty configs.
* **Documentation:** General code documentation improvements.

### Refactoring

* **Architecture:** - Major refactor of launcher-specific widgets; moved away from `mod.rs` into module-specific files.
  * Refactored `main.rs` into a dedicated `app` module.
  * Refactored `SherlockError` into `SherlockMessage` for better maintainability.
* **File Search:** Moved file search helpers to `utils.rs` and added model variants.
* **ContextMenu:** Moved `ContextMenuAction` to `context_menu.rs`.

### Fixed

* **Core:** Resolved `tokio-gpui` incompatibility by migrating to `smol`.
* **Deserialization:** Fixed incorrect deserialization/serialization of `ContextMenuActions` which caused application loading failures.
* **Emoji:** Fixed skin tone application issues and incorrect loading state.
* **Files:** Removed hardcoded home directory references.
* **Launchers:** Fixed launcher type migration and arg bar completion.
* **UI:** Fixed double borders, backgrounds, and incorrect icon/context menu rendering.
