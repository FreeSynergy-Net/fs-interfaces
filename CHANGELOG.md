# Changelog

Version = tag; all crates of the workspace (`fs-interfaces`, `fs-mcp`, `regenerate`) carry the same version. The Cap'n Proto schema and the capability list only grow (append only). Details: `DESIGN.md`.

## 0.0.27

- Origin: `fs-types` is moved out to FreeSynergy/fs-types v0.0.1 (history kept), see ADR 0093. Removed here: the workspace crate `fs-types`, `schema/types.capnp`, `src/types_capnp.rs`, the feature `values` (`EmailAddress`, `PhoneNumber`, `ValueProblem`, trait `ContactValue`) and the dependency `idna`. `fs-interfaces` depends on `fs-types` (git, tag `v0.0.1`).
- Breaking for programs: `fs_interfaces::values`, `fs_interfaces::types_capnp` and the value objects are gone, and nothing is re-exported. Pin `fs-types` directly (`fs-types = { git = "https://github.com/FreeSynergy/fs-types", tag = "v0.0.1" }`) and use `fs_types::...` and `fs_types::types_capnp`; the lock must hold one `fs-types`, from that repository only.
- 13 schemas (events, files, streaming, surveys, texts, tasks, groups, history, people, messenger, whiteboard, places, resources) import `"/types.capnp"` (import path `fs_types::SCHEMA_DIR`); the generated code refers to `::fs_types::types_capnp`. File id and type ids unchanged; the wire is unchanged; the generated files also differ in the source byte ranges embedded in the schema nodes (189 words in 13 files, shifted by one byte).
- `fs_types::Capabilities` stays in `fs-types` next to `fs_interfaces::provider::Capabilities` (V3 open); users: `person_directory_v1`, `tasks_v1`, `appointments_v1`.
- `generated_matches_official` (both compilers, 17 schemas) writes into the test target instead of `/tmp`. `fs-mcp`: `CapnpcSource::with_schemas` adds the schema directory of `fs-types`. All crates of the workspace carry 0.0.27.
- Auf Deutsch: `fs-types` ist nach FreeSynergy/fs-types v0.0.1 ausgelagert (ADR 0093, Historie erhalten). Entfallen: Crate `fs-types`, `schema/types.capnp`, `types_capnp`, Feature `values` (Wertobjekte) und `idna`. BREAKING fuer Programme: `fs_interfaces::values` und `fs_interfaces::types_capnp` gibt es nicht mehr, es gibt keinen Re-Export; `fs-types` direkt pinnen (Tag v0.0.1, nur diese Quelle im Lock). 13 Schemata importieren `"/types.capnp"`; Datei- und Typ-IDs und der Draht sind unveraendert.

## 0.0.26

- Breaking: the module `rpc` (38 source files) and the feature `rpc` are gone; the server and client runtime (Unix socket and named pipe, envelope, RFC 9457 problems as RPC errors, pages, change feed, version handshake, `Worker`) moves to the new library `fs-rpc` (ADR 0092 draft). Migration: `fs_interfaces::rpc::...` becomes `fs_rpc::...`.
- `fs-interfaces` no longer depends on `fs-objects` (the dependency was only used by `rpc`): it depends on none of our runtime repos, so the repo graph has no cycle (`fs-interfaces` to `fs-objects` to `fs-types` to `fs-interfaces`).
- The test `texts_contract` stays here without its `Problem` round-trip test (that one moved to `fs-rpc`, `tests/rpc_problem_texts.rs`).
- Nothing else changes: schemas, generated code, `provider`, `values` and `fs-types` stay as in 0.0.25. All crates of the workspace carry 0.0.26.
- Auf Deutsch: Das Modul `rpc` und das Feature `rpc` entfallen; die RPC-Laufzeit liegt jetzt in der Bibliothek `fs-rpc`. `fs-interfaces` haengt nicht mehr von `fs-objects` ab. Umstellung: `fs_interfaces::rpc::` wird `fs_rpc::`.

## 0.0.25

- `fs-interfaces` uses `fs-objects` v0.0.3 (was v0.0.1); consumers must follow with the lift (until then their locks hold two versions).
- `Location`: `name` optional (RFC 8984 section 4.2.5; `Location::new()` takes no name, `with_name`), new `relativeTo` (`RelativeTo`: start/end) and `timeZone` (`Zone`); `LocationParts` has private fields with a builder and getters (new JSCalendar fields later break nobody). Maps to `fs-jscalendar`'s `Location` object (not touched here). Open for v0.0.26: NFC for description, lower case for types, bounds, link `rel`/`contentType`/`title`.
- `fs-types`: new `Location` (JSCalendar `Location`, RFC 8984 section 4.2.5: name optional, description, location types, coordinates as `GeoPoint`, postal `Address`, links) with `LocationParts` for the JSCalendar mapping; no new wire kind (coordinates travel as `Place`).
- `fs-types`: `Place::parse` / `canonical` normalise through `GeoPoint` (`geo:0,180` is `geo:0,-180`, `geo:-0,-0` is `geo:0,0`); `Place` to `GeoPoint` to `Place` loses nothing now. A negative `u=` is `OutOfRange` for `Place` as for `GeoPoint` (was `BadPlace`).
- `AccuracyLevel` documentation: the AI sees at most the city (about 10 km); routing without an AI may be exact (rule lives in `fs-access`, not enforced by the type).

## 0.0.24

- `fs-types`: `GeoPoint` (WGS 84, `geo:` URI RFC 5870, GeoJSON helpers), `PositionFix` with `FixSource`, `AccuracyLevel` (blur to a grid; `at_level` always blurs, also with a forged `blur` field). `Place::point()` and `Place::from_point()`. Additive; wire format unchanged.

## 0.0.23

- Names wave (ADR 0087): owner and program names in comments, headers and documentation follow the renames; no capability key, Cap'n Proto type or contract changed.

## 0.0.22

- `fs-types`: the shared types of the topic crates (`Utc`, `LocalDateTime`, `Zone`, `PersonRef`, `PersonArt`, `Attendee`, `ContactAddress`, `ObjectId`, `Revision`, `MediaType`, `FileRef`, `Page`, `PageCursor`, `Capabilities`, `Problem`), `TypeProblem` as the union of the stand-in problems; `Reference` and `ContactAddress` refuse an empty rest; `Utc` range and leap second checked.
