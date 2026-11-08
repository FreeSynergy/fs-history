//! Hub activity -> `SourceActivity`.
//!
//! Pattern: Adapter — reads the envelope summary of the images (type, name, owner, context) and
//! nothing else. The images' content and the payload of log-only verbs are never copied. The hub
//! reports the program as `actor` and the time it applied the push, so `actor` stays empty here and
//! `at_is_received` is true.

use fs_history_core::{Error, Result, SourceActivity};
use fs_objects::ObjectId;
use fs_objects_hub::Activity;

fn bad(what: &str, e: impl std::fmt::Display) -> Error {
    Error::Source(format!("hub activity: {what}: {e}"))
}

/// The summary of one activity of the hub's log.
pub(crate) fn summarise(a: &Activity) -> Result<SourceActivity> {
    let mut out = SourceActivity {
        id: a.id.to_string().parse().map_err(|e| bad("id", e))?,
        at: a.at,
        program: a.actor.clone(),
        kind: a.kind.as_str().to_string(),
        object: a.object.to_string().parse().map_err(|e| bad("object", e))?,
        object_type: String::new(),
        name: String::new(),
        attributed_to: String::new(),
        context: String::new(),
        actor: String::new(),
        at_is_received: true,
    };
    // Log-only verbs carry no envelope; the core fills them from the object's latest entry.
    if a.kind.changes_state() {
        let images = a.change_set().map_err(|e| bad("change set", e))?;
        if let Some(env) = images.after.or(images.before).map(|o| o.envelope) {
            out.object_type = env.object_type;
            out.name = env.name;
            out.attributed_to = env.attributed_to.unwrap_or_default();
            out.context = env.context.unwrap_or_default();
        }
    }
    Ok(out)
}

/// The object id type of the core, from a hub activity id.
pub(crate) fn to_core_id(id: fs_objects_hub::ActivityId) -> Result<ObjectId> {
    id.to_string().parse().map_err(|e| bad("id", e))
}
