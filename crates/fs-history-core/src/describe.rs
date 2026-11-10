//! The description of the data types of history: what a generated frontend is told (ADR 0066).
//!
//! Pattern: Registry entry (trait first) + Adapter — [`Entry`] and [`DigestEntry`] implement `DescribesType` of
//! `fs-types`, so `TypeRegistry::register::<Entry>()` puts them in the catalog the program serves. Fields are the
//! members of `Entry` and `DigestEntry` in `history.capnp`; commands are built from the single table in
//! `commands.rs`, supported or not.
//!
//! Not described (named in `DESIGN.md`, `Unsupported`): `atIsReceived` and `continues` (no yes/no basic type yet),
//! `count` is a text with the decimal number (no number basic type yet).

use fs_types::{
    BasicType, CommandDescription, DescribesType, FieldDescription, TypeDescription, TypeRegistry,
};

use crate::commands::{self, CommandRow, DIGEST, ENTRY, FieldSpec};
use crate::model::{DigestEntry, Entry};

/// The program that owns the types (ADR 0050).
pub const PROGRAM: &str = "fs-history";

/// `objectType` -> `object-type`: the translation key part of a name.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn field(prefix: &str, name: &str, basic: BasicType) -> FieldDescription {
    FieldDescription::new(name, basic).label(&format!("{prefix}.{}", kebab(name)))
}

fn describe_spec(prefix: &str, s: &FieldSpec) -> FieldDescription {
    let mut d = field(prefix, s.name, s.basic);
    if s.required {
        d = d.required();
    }
    if s.multiple {
        d = d.multiple();
    }
    if !s.target.is_empty() {
        d = d.target(s.target);
    }
    if !s.choices.is_empty() {
        d = d.choices(s.choices);
    }
    d
}

fn describe_command(type_label: &str, r: &CommandRow) -> CommandDescription {
    let key = format!("{type_label}.{}", kebab(r.name));
    let mut c = CommandDescription::new(r.name, r.kind, &key);
    for f in &r.input {
        c = c.input(describe_spec(&key, f));
    }
    for f in &r.result {
        c = c.result(describe_spec(&key, f));
    }
    // SWITCH: set .supported(r.supported) when fs-types has it. Until then every command is in the description and the
    // flag is only in the table (`commands::unsupported_commands`) and in the refusal of the backend.
    c
}

fn with_commands(mut t: TypeDescription, name: &str, label: &str) -> TypeDescription {
    for r in commands::table().iter().filter(|r| r.data_type == name) {
        t = t.command(describe_command(label, r));
    }
    t
}

/// An entry is a summary of one AS2 activity; it never holds the content.
impl DescribesType for Entry {
    fn describe() -> TypeDescription {
        let l = "entry";
        let t = TypeDescription::new(ENTRY, PROGRAM, "AS2 Activity (summary)", l)
            .field(field(l, "id", BasicType::Text).required())
            .field(field(l, "at", BasicType::DateTime).required())
            .field(field(l, "program", BasicType::Text).required())
            .field(field(l, "kind", BasicType::Text).required())
            .field(field(l, "object", BasicType::Reference).required())
            .field(field(l, "objectType", BasicType::Text))
            .field(field(l, "name", BasicType::Text))
            .field(field(l, "attributedTo", BasicType::Reference))
            .field(field(l, "context", BasicType::Text))
            .field(field(l, "actor", BasicType::Reference));
        with_commands(t, ENTRY, l)
    }
}

/// Several close changes of one object by one program, as one line.
impl DescribesType for DigestEntry {
    fn describe() -> TypeDescription {
        let l = "digest";
        let t = TypeDescription::new(DIGEST, PROGRAM, "AS2 Activity (grouped summary)", l)
            .field(field(l, "first", BasicType::Reference).target(ENTRY).required())
            .field(field(l, "lastAt", BasicType::DateTime).required())
            .field(field(l, "count", BasicType::Text).required())
            .field(field(l, "kinds", BasicType::Text).multiple());
        with_commands(t, DIGEST, l)
    }
}

/// The registry history serves: `Entry` and `Digest`.
///
/// # Errors
/// A defect of a description (cannot happen with the table that is tested).
pub fn registry() -> Result<TypeRegistry, fs_types::DescriptionProblem> {
    let mut r = TypeRegistry::new();
    r.register::<Entry>()?.register::<DigestEntry>()?;
    Ok(r)
}
