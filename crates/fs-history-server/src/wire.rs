//! Entries, filters and cursors between the core and `history.capnp`.
//!
//! Pattern: Adapter — pure functions, one pair per wire struct.

use fs_history_core::{Cursor, DigestEntry, Entry, EntryFilter, Page, Timestamp};
use fs_interfaces::history_capnp::{digest_entry, digest_page, entry, entry_filter, entry_page};
use fs_interfaces::rpc::{
    from_millis, invalid_request, read_uuid, to_millis, write_object_id,
};
use fs_objects::ObjectId;

fn time(t: fs_interfaces::common_capnp::timestamp::Reader<'_>) -> capnp::Result<Option<Timestamp>> {
    let millis = t.get_unix_millis();
    // An all-zero time counts as unset (the contract says so).
    if millis == 0 {
        return Ok(None);
    }
    from_millis(millis).map(Some)
}

fn text(r: capnp::text::Reader<'_>) -> capnp::Result<String> {
    Ok(r.to_string()?)
}

/// Read a filter; all-zero members are unset.
pub(crate) fn read_filter(r: entry_filter::Reader<'_>) -> capnp::Result<EntryFilter> {
    let object = match read_uuid(r.get_object()?) {
        Some(u) => Some(ObjectId::from_uuid(u).map_err(|e| invalid_request(e.to_string()))?),
        None => None,
    };
    Ok(EntryFilter {
        from: time(r.get_from()?)?,
        to: time(r.get_to()?)?,
        program: text(r.get_program()?)?,
        person: text(r.get_person()?)?,
        object_type: text(r.get_object_type()?)?,
        context: text(r.get_context()?)?,
        object,
    })
}

/// The cursor the caller handed back; empty = start.
pub(crate) fn read_cursor(
    r: fs_interfaces::common_capnp::cursor::Reader<'_>,
) -> capnp::Result<Option<Cursor>> {
    let bytes = r.get_opaque()?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let s = std::str::from_utf8(bytes).map_err(|_| invalid_request("cursor is not one this server handed out"))?;
    Ok(Some(Cursor(s.to_owned())))
}

fn write_cursor(mut out: fs_interfaces::common_capnp::cursor::Builder<'_>, next: Option<&Cursor>) {
    if let Some(c) = next {
        out.set_opaque(c.0.as_bytes());
    }
}

/// Write an entry.
pub(crate) fn write_entry(mut out: entry::Builder<'_>, e: &Entry) {
    write_object_id(out.reborrow().init_id(), e.id);
    out.reborrow().init_at().set_unix_millis(to_millis(e.at));
    out.set_program(e.program.as_str());
    out.set_kind(e.kind.as_str());
    write_object_id(out.reborrow().init_object(), e.object);
    out.set_object_type(e.object_type.as_str());
    out.set_name(e.name.as_str());
    out.set_attributed_to(e.attributed_to.as_str());
    out.set_context(e.context.as_str());
    out.set_actor(e.actor.as_str());
    out.set_at_is_received(e.at_is_received);
}

/// Write a page of entries.
pub(crate) fn write_entries(mut out: entry_page::Builder<'_>, page: &Page<Entry>) {
    let mut list = out
        .reborrow()
        .init_items(u32::try_from(page.items.len()).unwrap_or(u32::MAX));
    for (i, e) in (0u32..).zip(&page.items) {
        write_entry(list.reborrow().get(i), e);
    }
    write_cursor(out.init_next(), page.next.as_ref());
}

fn write_digest(mut out: digest_entry::Builder<'_>, d: &DigestEntry) {
    write_entry(out.reborrow().init_first(), &d.first);
    out.reborrow().init_last_at().set_unix_millis(to_millis(d.last_at));
    out.set_count(d.count);
    let mut kinds = out
        .reborrow()
        .init_kinds(u32::try_from(d.kinds.len()).unwrap_or(u32::MAX));
    for (i, k) in (0u32..).zip(&d.kinds) {
        kinds.set(i, k.as_str());
    }
    out.set_continues(d.continues);
}

/// Write a page of digest lines.
pub(crate) fn write_digests(mut out: digest_page::Builder<'_>, page: &Page<DigestEntry>) {
    let mut list = out
        .reborrow()
        .init_items(u32::try_from(page.items.len()).unwrap_or(u32::MAX));
    for (i, d) in (0u32..).zip(&page.items) {
        write_digest(list.reborrow().get(i), d);
    }
    write_cursor(out.init_next(), page.next.as_ref());
}
