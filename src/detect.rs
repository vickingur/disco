//! Developer-artifact detection.
//!
//! A project is identified by a *marker file* in a directory (e.g. `Cargo.toml`);
//! each project type maps to the *artifact directories* under that root which are
//! safe to delete and regenerable. This marker→artifacts table is adapted from
//! kondo (MIT, © 2020 Trent Billington); see ATTRIBUTION.md.
//!
//! Beyond kondo we add `pyvenv.cfg`-based virtualenv detection (the venv directory
//! itself is the cleanable unit), since that's a primary target for this tool.

/// A marker that identifies a project root by inspecting a directory's file names.
enum Marker {
    /// Exact file name match, e.g. `Cargo.toml`.
    File(&'static str),
    /// File-name suffix match, e.g. `.csproj`.
    Suffix(&'static str),
}

/// A kind of project and the regenerable directories it produces.
pub struct ProjectKind {
    /// Display name, e.g. `Cargo`, `Node`.
    pub name: &'static str,
    /// Any one of these in a directory marks it as this kind of project root.
    markers: &'static [Marker],
    /// Directory names (relative to the project root) that are cleanable artifacts.
    pub artifact_dirs: &'static [&'static str],
}

/// File name whose presence in a directory means that directory is a Python
/// virtualenv; the directory itself is then the cleanable artifact.
pub const VENV_MARKER: &str = "pyvenv.cfg";

/// Display name used for detected virtualenvs.
pub const VENV_KIND: &str = "Python venv";

/// The project-detection table, adapted from kondo. Order matters only for
/// reporting ties; detection returns the first kind whose marker matches.
pub static PROJECT_KINDS: &[ProjectKind] = &[
    ProjectKind {
        name: "Cargo",
        markers: &[Marker::File("Cargo.toml")],
        artifact_dirs: &["target", ".xwin-cache"],
    },
    ProjectKind {
        name: "Node",
        markers: &[Marker::File("package.json")],
        artifact_dirs: &["node_modules", ".angular"],
    },
    ProjectKind {
        name: "Turborepo",
        markers: &[Marker::File("turbo.json")],
        artifact_dirs: &[".turbo"],
    },
    ProjectKind {
        name: "Unity",
        markers: &[Marker::File("Assembly-CSharp.csproj")],
        artifact_dirs: &[
            "Library",
            "Temp",
            "Obj",
            "Logs",
            "MemoryCaptures",
            "Build",
            "Builds",
        ],
    },
    ProjectKind {
        name: "Stack",
        markers: &[Marker::File("stack.yaml")],
        artifact_dirs: &[".stack-work"],
    },
    ProjectKind {
        name: "Cabal",
        markers: &[Marker::File("cabal.project")],
        artifact_dirs: &["dist-newstyle"],
    },
    ProjectKind {
        name: "SBT",
        markers: &[Marker::File("build.sbt")],
        artifact_dirs: &["target", "project/target"],
    },
    ProjectKind {
        name: "Maven",
        markers: &[Marker::File("pom.xml")],
        artifact_dirs: &["target"],
    },
    ProjectKind {
        name: "Gradle",
        markers: &[
            Marker::File("build.gradle"),
            Marker::File("build.gradle.kts"),
        ],
        artifact_dirs: &["build", ".gradle"],
    },
    ProjectKind {
        name: "CMake",
        markers: &[Marker::File("CMakeLists.txt")],
        artifact_dirs: &["build", "cmake-build-debug", "cmake-build-release"],
    },
    ProjectKind {
        name: "Unreal",
        markers: &[Marker::Suffix(".uproject")],
        artifact_dirs: &[
            "Binaries",
            "Build",
            "Saved",
            "DerivedDataCache",
            "Intermediate",
        ],
    },
    ProjectKind {
        name: "Jupyter",
        markers: &[Marker::Suffix(".ipynb")],
        artifact_dirs: &[".ipynb_checkpoints"],
    },
    ProjectKind {
        name: "Python",
        markers: &[Marker::Suffix(".py")],
        artifact_dirs: &[
            ".mypy_cache",
            ".nox",
            ".pytest_cache",
            ".ruff_cache",
            ".tox",
            "__pycache__",
            "__pypackages__",
        ],
    },
    ProjectKind {
        name: "Pixi",
        markers: &[Marker::File("pixi.toml")],
        artifact_dirs: &[".pixi"],
    },
    ProjectKind {
        name: "Composer",
        markers: &[Marker::File("composer.json")],
        artifact_dirs: &["vendor"],
    },
    ProjectKind {
        name: "Pub",
        markers: &[Marker::File("pubspec.yaml")],
        artifact_dirs: &[
            "build",
            ".dart_tool",
            "linux/flutter/ephemeral",
            "windows/flutter/ephemeral",
        ],
    },
    ProjectKind {
        name: "Elixir",
        markers: &[Marker::File("mix.exs")],
        artifact_dirs: &["_build", ".elixir-tools", ".elixir_ls", ".lexical"],
    },
    ProjectKind {
        name: "Swift",
        markers: &[Marker::File("Package.swift")],
        artifact_dirs: &[".build", ".swiftpm"],
    },
    ProjectKind {
        name: "Zig",
        markers: &[Marker::File("build.zig")],
        artifact_dirs: &["zig-cache", ".zig-cache", "zig-out"],
    },
    ProjectKind {
        name: "Godot 4.x",
        markers: &[Marker::File("project.godot")],
        artifact_dirs: &[".godot"],
    },
    ProjectKind {
        name: ".NET",
        markers: &[Marker::Suffix(".csproj"), Marker::Suffix(".fsproj")],
        artifact_dirs: &["bin", "obj"],
    },
    ProjectKind {
        name: "Terraform",
        markers: &[Marker::File(".terraform.lock.hcl")],
        artifact_dirs: &[".terraform"],
    },
    ProjectKind {
        name: "CocoaPods",
        markers: &[Marker::File("Podfile")],
        artifact_dirs: &["Pods"],
    },
];

impl Marker {
    fn matches(&self, file_name: &str) -> bool {
        match self {
            Marker::File(name) => file_name == *name,
            Marker::Suffix(suffix) => file_name.ends_with(suffix),
        }
    }
}

/// Given the file names directly inside a directory, return the project kind it
/// is the root of, if any. The first matching kind in [`PROJECT_KINDS`] wins, so
/// more specific markers (`Assembly-CSharp.csproj`) are listed before broader ones
/// (`.csproj`).
pub fn project_kind(file_names: &[&str]) -> Option<&'static ProjectKind> {
    PROJECT_KINDS.iter().find(|kind| {
        kind.markers
            .iter()
            .any(|marker| file_names.iter().any(|name| marker.matches(name)))
    })
}

/// Whether a directory is itself a Python virtualenv (contains `pyvenv.cfg`).
pub fn is_venv(file_names: &[&str]) -> bool {
    file_names.contains(&VENV_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_marker_detected() {
        let kind = project_kind(&["Cargo.toml", "src"]).expect("cargo project");
        assert_eq!(kind.name, "Cargo");
        assert!(kind.artifact_dirs.contains(&"target"));
    }

    #[test]
    fn specific_csproj_marker_wins_over_generic() {
        let kind = project_kind(&["Assembly-CSharp.csproj"]).expect("unity project");
        assert_eq!(kind.name, "Unity");
    }

    #[test]
    fn generic_csproj_is_dotnet() {
        let kind = project_kind(&["App.csproj"]).expect("dotnet project");
        assert_eq!(kind.name, ".NET");
    }

    #[test]
    fn python_detected_by_py_suffix() {
        let kind = project_kind(&["main.py"]).expect("python project");
        assert_eq!(kind.name, "Python");
    }

    #[test]
    fn venv_detected_by_marker_file() {
        assert!(is_venv(&["pyvenv.cfg", "bin", "lib"]));
        assert!(!is_venv(&["requirements.txt"]));
    }

    #[test]
    fn no_marker_no_project() {
        assert!(project_kind(&["README.md", "notes.txt"]).is_none());
    }
}
