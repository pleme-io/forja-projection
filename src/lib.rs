//! `forja-projection` — **THE one seam** of the pleme-io generation substrate.
//! A typed **primary
//! structure** projects every artifact through ONE trait, so a new target /
//! backend / platform is a new [`Projection`] impl and a catalog row — **nothing
//! else**.
//!
//! The fleet rediscovered this shape four times — iac-forge `Backend`,
//! hata-emit `Projection`, repo-forge `Synthesizer`, caixa's no-trait
//! copy-paste — all `fn target() + fn project(source) -> Result<artifact>`.
//! Per Operating Principle #1 (fix the near-miss, don't duplicate) they repoint
//! onto this ONE seam. **Distinct primary structures share only the seam, never
//! the source type** — `IacResource ≠ DefEntidade ≠ WidgetSpec ≠ Caixa` — this
//! is the bright line: do not force-merge what should stay distinct.
//!
//! Every [`GeneratedArtifact`] carries a BLAKE3 `content_hash` computed on
//! construction — the content-address drift detection uses. `source_hash` (the
//! *primary-structure* provenance) is an OPTIONAL richer-consumer tier
//! (e.g. iac-forge's morphism chain); **hata sets only `content_hash` today** —
//! the provenance link is wired where a consumer needs it, not claimed by default.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// The target syntax family of an artifact — a **typed** tag (never a stringly
/// kind). A target-syntax renderer keys its `format!()`-ban exception off this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// Rust source (emitted via `quote!`→`syn`→prettyplease).
    RustSource,
    /// GraphQL SDL.
    GraphqlSdl,
    /// SQL DDL.
    SqlDdl,
    /// An OpenAPI document.
    OpenApi,
    /// Nix source (a typed `NixValue` render).
    Nix,
    /// YAML (a typed `serde_yaml::Value` render).
    Yaml,
    /// JSON (a typed `serde_json` render).
    Json,
    /// Go source.
    Go,
    /// Swift source.
    Swift,
    /// Kotlin source.
    Kotlin,
    /// Any other target syntax, named.
    Other(String),
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactKind::RustSource => f.write_str("rust"),
            ArtifactKind::GraphqlSdl => f.write_str("graphql-sdl"),
            ArtifactKind::SqlDdl => f.write_str("sql-ddl"),
            ArtifactKind::OpenApi => f.write_str("openapi"),
            ArtifactKind::Nix => f.write_str("nix"),
            ArtifactKind::Yaml => f.write_str("yaml"),
            ArtifactKind::Json => f.write_str("json"),
            ArtifactKind::Go => f.write_str("go"),
            ArtifactKind::Swift => f.write_str("swift"),
            ArtifactKind::Kotlin => f.write_str("kotlin"),
            ArtifactKind::Other(s) => f.write_str(s),
        }
    }
}

/// One emitted artifact + its content-address. `content_hash` (BLAKE3 of
/// `content`) is computed on construction — every artifact is content-addressed
/// for free (drift / freshness / provenance). `source_hash` is the OPTIONAL
/// provenance of the primary structure that produced it (set by richer
/// consumers — e.g. iac-forge's morphism chain).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedArtifact {
    /// The logical (relative) path / name of the artifact.
    pub path: String,
    /// The emitted source.
    pub content: String,
    /// The target syntax family.
    pub kind: ArtifactKind,
    /// BLAKE3 of `content` — the content-address.
    pub content_hash: [u8; 32],
    /// BLAKE3 of the primary structure that produced this (optional provenance).
    pub source_hash: Option<[u8; 32]>,
}

impl GeneratedArtifact {
    /// Build an artifact, computing its BLAKE3 content-address on construction.
    #[must_use]
    pub fn new(path: impl Into<String>, content: impl Into<String>, kind: ArtifactKind) -> Self {
        let content = content.into();
        let content_hash = hash_bytes(content.as_bytes());
        Self { path: path.into(), content, kind, content_hash, source_hash: None }
    }

    /// Attach the provenance hash of the primary structure (the optional tier).
    #[must_use]
    pub fn with_source_hash(mut self, source_hash: [u8; 32]) -> Self {
        self.source_hash = Some(source_hash);
        self
    }

    /// The content-address as a lowercase-hex string.
    #[must_use]
    pub fn content_hash_hex(&self) -> String {
        hex(&self.content_hash)
    }
}

/// BLAKE3 of an arbitrary byte slice — the fleet content-address primitive.
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

/// Lowercase-hex of a 32-byte digest, via a typed nibble lookup — no `format!()`
/// (TYPED EMISSION).
fn hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// A typed emission error — never a silent wrong answer (TYPED-SPEC TRIPLET).
#[derive(Debug, thiserror::Error)]
#[error("projection emission failed: {0}")]
pub struct EmitError(String);

impl EmitError {
    /// Construct an emission error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// **THE seam:** a projection of a typed primary structure `S` into one or more
/// generated artifacts. A new target /
/// backend / platform is one `impl Projection<S>` + a catalog row — nothing
/// else. The four fleet rediscoveries repoint onto this; distinct primary
/// structures share ONLY this seam, never the source type.
pub trait Projection<S: ?Sized> {
    /// The target name — the registry key / diagnostics label.
    fn target(&self) -> &'static str;

    /// Project the primary structure into its artifact(s).
    ///
    /// # Errors
    /// [`EmitError`] if emission fails.
    fn project(&self, source: &S) -> Result<Vec<GeneratedArtifact>, EmitError>;
}

/// **The weave primitive:** run a whole registry of projections over ONE source,
/// collecting `target → artifacts`. This is the loop every orchestrator would
/// otherwise hand-roll (it appeared twice — backend + frontend — in forge-hata,
/// the third-use trigger). Encapsulated here so the registry-weave is solved
/// once, on the seam, for every consumer (Prime Directive: duplication is a bug).
///
/// All-or-nothing: the first failing projection aborts the whole weave with its
/// typed [`EmitError`] — never a silent partial.
///
/// # Errors
/// [`EmitError`] from the first projection that fails.
pub fn project_all<S: ?Sized>(
    registry: &[Box<dyn Projection<S>>],
    source: &S,
) -> Result<BTreeMap<String, Vec<GeneratedArtifact>>, EmitError> {
    let mut out = BTreeMap::new();
    for p in registry {
        let target = p.target().to_string();
        let artifacts = p.project(source)?;
        // A duplicate target would silently clobber the first projection's
        // artifacts (last-write-wins) — a typed error instead, never a silent drop.
        if out.insert(target.clone(), artifacts).is_some() {
            return Err(EmitError::new(["duplicate projection target: ", &target].concat()));
        }
    }
    Ok(out)
}

/// **Generate a whole projection registry from a table** — the Prime Directive
/// applied to the seam. Each row `Struct => "target", path, body, kind;` emits a
/// unit `struct`, its `impl Projection<S>` (one content-addressed artifact), and
/// at the end a `registry()` fn returning every projection boxed. The body is an
/// expression with `source: &S` in scope (so it may use `?` for a fallible
/// emitter); `path`/`kind` are expressions too (so a dynamic path or
/// `ArtifactKind::Other(..)` both work). Realizes the trait's own promise: "a new
/// target is one row — nothing else."
///
/// The `source = T as <name>` binds the source parameter to `<name>`, which the
/// `path`/`body` expressions then reference (so macro hygiene lines up).
///
/// ```ignore
/// define_projection! {
///     source = [DefEntidade] as src;
///     registry = backend_projections;
///     GraphqlSdl => "graphql-sdl", "schema.graphql", emit_entities_sdl(src), ArtifactKind::GraphqlSdl;
///     SeaOrm     => "seaorm",      "entities.rs",    seaorm::emit_entities_seaorm(src)?, ArtifactKind::RustSource;
/// }
/// ```
#[macro_export]
macro_rules! define_projection {
    (
        source = $src:ty as $bind:ident;
        registry = $registry:ident;
        $( $proj:ident => $target:literal, $path:expr, $body:expr, $kind:expr ; )+
    ) => {
        $(
            /// A projection generated by [`define_projection!`].
            pub struct $proj;
            impl $crate::Projection<$src> for $proj {
                fn target(&self) -> &'static str { $target }
                fn project(
                    &self,
                    $bind: &$src,
                ) -> ::core::result::Result<::std::vec::Vec<$crate::GeneratedArtifact>, $crate::EmitError> {
                    ::core::result::Result::Ok(::std::vec![$crate::GeneratedArtifact::new($path, $body, $kind)])
                }
            }
        )+
        /// The projection registry generated by [`define_projection!`].
        #[must_use]
        pub fn $registry() -> ::std::vec::Vec<::std::boxed::Box<dyn $crate::Projection<$src>>> {
            ::std::vec![ $( ::std::boxed::Box::new($proj) ),+ ]
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_is_content_addressed_on_construction() {
        let a = GeneratedArtifact::new("schema.sql", "CREATE TABLE x ();", ArtifactKind::SqlDdl);
        // Deterministic content-address: same bytes → same hash.
        let b = GeneratedArtifact::new("other/path.sql", "CREATE TABLE x ();", ArtifactKind::SqlDdl);
        assert_eq!(a.content_hash, b.content_hash, "content-address must depend only on content");
        // Different content → different hash.
        let c = GeneratedArtifact::new("schema.sql", "CREATE TABLE y ();", ArtifactKind::SqlDdl);
        assert_ne!(a.content_hash, c.content_hash);
        assert_eq!(a.content_hash_hex().len(), 64);
        assert!(a.source_hash.is_none());
    }

    #[test]
    fn source_hash_is_an_optional_provenance_tier() {
        let h = hash_bytes(b"(defentidade :nome Drive)");
        let a = GeneratedArtifact::new("entities.rs", "pub struct Model;", ArtifactKind::RustSource)
            .with_source_hash(h);
        assert_eq!(a.source_hash, Some(h));
    }

    #[test]
    fn artifact_kind_renders_a_typed_tag() {
        assert_eq!(ArtifactKind::GraphqlSdl.to_string(), "graphql-sdl");
        assert_eq!(ArtifactKind::Other("kcl".into()).to_string(), "kcl");
    }

    // A toy consumer proving the seam works over an arbitrary primary structure.
    struct UpperShout;
    impl Projection<str> for UpperShout {
        fn target(&self) -> &'static str {
            "upper-shout"
        }
        fn project(&self, source: &str) -> Result<Vec<GeneratedArtifact>, EmitError> {
            if source.is_empty() {
                return Err(EmitError::new("empty source"));
            }
            Ok(vec![GeneratedArtifact::new("out.txt", source.to_uppercase(), ArtifactKind::Other("text".into()))])
        }
    }

    struct LenReport;
    impl Projection<str> for LenReport {
        fn target(&self) -> &'static str {
            "len-report"
        }
        fn project(&self, source: &str) -> Result<Vec<GeneratedArtifact>, EmitError> {
            // typed: number → string via Display, no format!()
            Ok(vec![GeneratedArtifact::new("len.txt", source.len().to_string(), ArtifactKind::Other("text".into()))])
        }
    }

    #[test]
    fn a_projection_projects_a_primary_structure_into_artifacts() {
        let arts = UpperShout.project("drive").unwrap();
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].content, "DRIVE");
        assert_eq!(UpperShout.target(), "upper-shout");
        // A boxed trait object over the seam (the registry shape).
        let reg: Vec<Box<dyn Projection<str>>> = vec![Box::new(UpperShout)];
        assert!(reg[0].project("x").is_ok());
        assert!(UpperShout.project("").is_err());
    }

    #[test]
    fn project_all_weaves_a_registry_over_one_source() {
        let registry: Vec<Box<dyn Projection<str>>> = vec![Box::new(UpperShout), Box::new(LenReport)];
        let woven = project_all(&registry, "drive").unwrap();
        // target → artifacts, one entry per registered projection (BTreeMap → sorted).
        assert_eq!(woven.keys().map(String::as_str).collect::<Vec<_>>(), ["len-report", "upper-shout"]);
        assert_eq!(woven["upper-shout"][0].content, "DRIVE");
        assert_eq!(woven["len-report"][0].content, "5");
        // All-or-nothing: a failing projection aborts the whole weave (UpperShout errs on "").
        assert!(project_all(&registry, "").is_err());
    }

    // Generate two projections + a registry from a table — the macro under test.
    define_projection! {
        source = str as src;
        registry = toy_registry;
        Shout => "shout", "out.txt", src.to_uppercase(), ArtifactKind::Other("text".into());
        Len   => "len",   "len.txt", src.len().to_string(), ArtifactKind::Other("text".into());
    }

    #[test]
    fn define_projection_generates_the_impls_and_the_registry() {
        // The generated impls behave exactly like a hand-written Projection.
        assert_eq!(Shout.target(), "shout");
        assert_eq!(Shout.project("hi").unwrap()[0].content, "HI");
        assert_eq!(Len.project("hello").unwrap()[0].content, "5");
        // The generated registry weaves through project_all unchanged.
        let woven = project_all(&toy_registry(), "drive").unwrap();
        assert_eq!(woven.keys().map(String::as_str).collect::<Vec<_>>(), ["len", "shout"]);
        assert_eq!(woven["shout"][0].content, "DRIVE");
    }

    #[test]
    fn project_all_rejects_a_duplicate_target_instead_of_silently_dropping() {
        // Two projections with the same target() would clobber (last-write-wins) —
        // now a typed error, never a silent drop.
        let dup: Vec<Box<dyn Projection<str>>> = vec![Box::new(UpperShout), Box::new(UpperShout)];
        let err = project_all(&dup, "x").unwrap_err();
        assert!(err.to_string().contains("duplicate projection target"), "{err}");
    }
}
