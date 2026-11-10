# History — Requirements

What `FreeSynergy.History` must and can do, derived from the [frontends](frontends/index.md)
(ADR 0006, 0013). Each capability names its data standard (ADR 0014) and our status.

Status: `implemented` · `planned` · `proposed`

## Required (common to the analysed programs)

| Capability | Seen in | Data / standard | Status |
|------------|---------|-----------------|--------|

## Optional (notable ideas)

| Capability | Seen in | Data / standard | Status |
|------------|---------|-----------------|--------|
| Place and weather recorded with an entry | Day One | place (GeoJSON), weather as plain fields | proposed |
| "On this day": show events of the same date in earlier years | Day One | timeline query by date | proposed |
| Automatic capture of activity, raw events kept, summaries on demand | ActivityWatch | event buckets (JSON, RFC 3339 time) | proposed |
| Categorisation rules applied after capture | ActivityWatch | rule list (regular expressions) | proposed |
| Import of events from other programs through one local interface | ActivityWatch | HTTP/JSON, OpenTelemetry candidate | proposed |
| Correlations between unrelated data series | Exist.io | numeric series per day | proposed |
| Weekly summary | Exist.io | digest | proposed |
| Suggest duplicates of one person and merge them one by one or all at once; merging can be undone (`contact.mergeSuggest`, `contact.merge`) | Google Historys (Merge & fix); Apple Historys (merge or hide duplicates) | JSHistory RFC 9553 (merge result is a new card version); history via AS2 `Update` | proposed |
| Exchange own contact details in person, with a choice of which fields go out; receive-only possible; never overwrites an existing contact silently (`contact.shareNearby`) | Apple Historys (NameDrop) | JSHistory subset of the own card; transport open (ADR 0059 local transport) ⚠️ | proposed |
| Keep addresses of people only written to apart from the address book until promoted (`contact.promote`) | Google Historys (Other contacts) | JSHistory `kind`, vendor property `freesynergy.net:known` ⚠️ | proposed |
| Change history of a contact, with undo (`contact.history`) | Google Historys (History) | AS2 `Update` log; history data area not yet defined ⚠️ | proposed |
| Remind to get back in touch; show when a person was last spoken to (`contact.keepInTouch`) | Monica (reminders, "last spoke") | JSCalendar `Task` / alert; last contact derived from the message and event log | proposed |
| Record life events and milestones of a person (`contact.lifeEvent`) | Monica | JSCalendar `Event` linked by `relatedTo` ⚠️ | proposed |

## Not planned (deliberately)
