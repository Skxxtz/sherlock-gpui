#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Folder,
    Link,
    Code,
    Script,
    Web,
    Data,
    Text,
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Package,
    Font,
    Binary,
    Other,
}

impl FileCategory {
    /// Plain text that can be shown in a text preview.
    pub const fn is_text(self) -> bool {
        matches!(
            self,
            Self::Code | Self::Script | Self::Web | Self::Data | Self::Text
        )
    }
    pub const fn is_media(self) -> bool {
        matches!(self, Self::Image | Self::Audio | Self::Video)
    }
}

macro_rules! define_file_types {
    (
        $(
            $category:ident {
                $( $variant:ident => $label:literal [ $($ext:literal),* $(,)? ] ),* $(,)?
            }
        )*
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum FileKind {
            SymbolicLink,
            Folder,
            $( $( $variant, )* )*
            File,
        }

        impl FileKind {
            pub const fn as_str(self) -> &'static str {
                match self {
                    Self::SymbolicLink => "Symbolic Link",
                    Self::Folder => "Folder",
                    $( $( Self::$variant => $label, )* )*
                    Self::File => "File",
                }
            }

            pub const fn category(self) -> FileCategory {
                match self {
                    Self::SymbolicLink => FileCategory::Link,
                    Self::Folder => FileCategory::Folder,
                    $( $( Self::$variant => FileCategory::$category, )* )*
                    Self::File => FileCategory::Other,
                }
            }

            pub fn from_extension(extension: Option<&str>, is_symlink: bool, is_dir: bool) -> Self {
                if is_symlink { return Self::SymbolicLink; }
                if is_dir { return Self::Folder; }
                match extension.unwrap_or("").to_ascii_lowercase().as_str() {
                    $( $( $($ext)|* => Self::$variant, )* )*
                    _ => Self::File,
                }
            }
        }
    };
}

define_file_types! {
    Code {
        Rust              => "Rust"              ["rs"],
        C                 => "C"                 ["c"],
        CHeader           => "C Header"          ["h"],
        Cpp               => "C++"               ["cpp", "cc", "cxx"],
        CppHeader         => "C++ Header"        ["hpp", "hxx"],
        Go                => "Go"                ["go"],
        Zig               => "Zig"               ["zig"],
        Assembly          => "Assembly"          ["s", "asm"],
        Java              => "Java"              ["java"],
        Kotlin            => "Kotlin"            ["kt", "kts"],
        Scala             => "Scala"             ["scala"],
    }
    Script {
        Python            => "Python"            ["py"],
        Ruby              => "Ruby"              ["rb"],
        Lua               => "Lua"               ["lua"],
        Perl              => "Perl"              ["pl", "pm"],
        ShellScript       => "Shell Script"      ["sh"],
        BashScript        => "Bash Script"       ["bash"],
        ZshScript         => "Zsh Script"        ["zsh"],
        FishScript        => "Fish Script"       ["fish"],
        PowerShellScript  => "PowerShell Script" ["ps1"],
    }
    Web {
        JavaScript        => "JavaScript"        ["js", "mjs", "cjs"],
        TypeScript        => "TypeScript"        ["ts", "mts"],
        ReactJsx          => "React JSX"         ["jsx"],
        ReactTsx          => "React TSX"         ["tsx"],
        Html              => "HTML"              ["html", "htm"],
        Css               => "CSS"               ["css"],
        Scss              => "SCSS"              ["scss", "sass"],
        VueComponent      => "Vue Component"     ["vue"],
        SvelteComponent   => "Svelte Component"  ["svelte"],
    }
    Data {
        Json              => "JSON"              ["json", "jsonc"],
        Yaml              => "YAML"              ["yaml", "yml"],
        Toml              => "TOML"              ["toml"],
        Config            => "Config"            ["ini", "cfg", "conf"],
        EnvironmentConfig => "Environment Config" ["env"],
        Xml               => "XML"               ["xml"],
        Csv               => "CSV"               ["csv"],
        Tsv               => "TSV"               ["tsv"],
        Sql               => "SQL"               ["sql"],
    }
    Text {
        Markdown          => "Markdown"          ["md", "markdown"],
        ReStructuredText  => "reStructuredText"  ["rst"],
        LaTeX             => "LaTeX"             ["tex"],
        PlainText         => "Plain Text"        ["txt"],
        Log               => "Log"               ["log"],
        LockFile          => "Lock File"         ["lock"],
        PidFile           => "PID File"          ["pid"],
    }
    Document {
        Pdf               => "PDF"               ["pdf"],
        WordDocument      => "Word Document"     ["doc", "docx"],
        ExcelSpreadsheet  => "Excel Spreadsheet" ["xls", "xlsx"],
        PowerPoint        => "PowerPoint"        ["ppt", "pptx"],
        OpenDocumentText  => "OpenDocument Text" ["odt"],
        OpenDocumentSheet => "OpenDocument Spreadsheet" ["ods"],
        EBook             => "eBook"             ["epub"],
    }
    Image {
        JpegImage         => "JPEG Image"        ["jpg", "jpeg"],
        PngImage          => "PNG Image"         ["png"],
        GifImage          => "GIF Image"         ["gif"],
        WebpImage         => "WebP Image"        ["webp"],
        AvifImage         => "AVIF Image"        ["avif"],
        Svg               => "SVG"               ["svg"],
        Icon              => "Icon"              ["ico"],
        Bitmap            => "Bitmap"            ["bmp"],
        TiffImage         => "TIFF Image"        ["tiff", "tif"],
        HeifImage         => "HEIF Image"        ["heic", "heif"],
        RawImage          => "RAW Image"         ["raw", "cr2", "nef"],
    }
    Audio {
        Mp3               => "MP3"               ["mp3"],
        Flac              => "FLAC"              ["flac"],
        Wav               => "WAV"               ["wav"],
        Ogg               => "OGG"               ["ogg"],
        Aac               => "AAC"               ["aac"],
        M4a               => "M4A"               ["m4a"],
        Opus              => "Opus"              ["opus"],
    }
    Video {
        Mp4               => "MP4"               ["mp4", "m4v"],
        Mkv               => "MKV"               ["mkv"],
        Webm              => "WebM"              ["webm"],
        Avi               => "AVI"               ["avi"],
        QuickTime         => "QuickTime"         ["mov"],
        Wmv               => "WMV"               ["wmv"],
    }
    Archive {
        ZipArchive        => "ZIP Archive"       ["zip"],
        TarArchive        => "Tar Archive"       ["tar"],
        GzipArchive       => "Gzip Archive"      ["gz", "tgz"],
        XzArchive         => "XZ Archive"        ["xz"],
        Bzip2Archive      => "Bzip2 Archive"     ["bz2"],
        ZstandardArchive  => "Zstandard Archive" ["zst"],
        SevenZipArchive   => "7-Zip Archive"     ["7z"],
        RarArchive        => "RAR Archive"       ["rar"],
        JavaArchive       => "Java Archive"      ["jar"],
    }
    Package {
        DebianPackage     => "Debian Package"    ["deb"],
        RpmPackage        => "RPM Package"       ["rpm"],
        AppImage          => "AppImage"          ["appimage"],
        Flatpak           => "Flatpak"           ["flatpak"],
    }
    Font {
        TrueTypeFont      => "TrueType Font"     ["ttf"],
        OpenTypeFont      => "OpenType Font"     ["otf"],
        WebFont           => "Web Font"          ["woff", "woff2"],
    }
    Binary {
        SharedLibrary     => "Shared Library"    ["so"],
        StaticLibrary     => "Static Library"    ["a"],
        ObjectFile        => "Object File"       ["o"],
        Executable        => "Executable"        ["exe"],
        Dll               => "DLL"               ["dll"],
        WebAssembly       => "WebAssembly"       ["wasm"],
        JavaBytecode      => "Java Bytecode"     ["class"],
        SqliteDatabase    => "SQLite Database"   ["db", "sqlite", "sqlite3"],
    }
}

impl FileKind {
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Folder)
    }
    pub const fn is_image(self) -> bool {
        matches!(self.category(), FileCategory::Image)
    }
    pub const fn is_audio(self) -> bool {
        matches!(self.category(), FileCategory::Audio)
    }
    pub const fn is_video(self) -> bool {
        matches!(self.category(), FileCategory::Video)
    }
    pub const fn is_text(self) -> bool {
        self.category().is_text()
    }
    pub const fn is_archive(self) -> bool {
        matches!(self.category(), FileCategory::Archive)
    }
    /// Decodable Images
    pub const fn has_image_preview(self) -> bool {
        matches!(
            self,
            Self::JpegImage
                | Self::PngImage
                | Self::GifImage
                | Self::WebpImage
                | Self::Svg
                | Self::Icon
                | Self::Bitmap
                | Self::TiffImage
        )
    }
}
