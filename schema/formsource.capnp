# DRAFT contract `FormSource` (owner fs-forms; PLAN-quelle-und-abgleich section 2, order SQ-1b). Every module service serves it next to its
# own contract, the way it serves `Control`: the host (fs-starter) finds the service, checks it and connects; fs-forms draws by the
# form (`schema/forms.txt`). The form names are NOT part of this file (they are data in `forms.txt`, read by `fs_interfaces::forms`).
# Handshake: `hello` answers a plain level number (`FORM_SOURCE_LEVEL` in `fs_interfaces::forms`), never a name like `_v2`. A change that
# breaks a reader raises the number; a new method is appended. Evolution (Cap'n Proto): only append, never renumber or remove.
# Answers are foreign data for the caller: it keeps its own limits (page size, bytes, time) and masks control characters before drawing.
# A service that serves example data says `provider = "fake"`; the window then shows "example data" and never passes it off as real.

@0xfa9420fd361f284b;

using Common = import "common.capnp";

enum HiddenReason {
  noRight   @0;                                  # the asking person has no right to the field
  sensitive @1;                                  # the field is sensitive
}

struct HiddenField { path @0 :Text; reason @1 :HiddenReason; }
# Pattern: Command – ein Schreibbefehl ist ein Wert (Vertragsschlüssel, Datensatz, Argumente nach Feldnamen), den das Modul prüft und ausführt.
# Abweichung: Level 1 ohne Kennung, Erwartung und Rückgängig; das kommt angehängt mit Level 2 (invokeIf/revert/StepRef).
struct Value       { path @0 :Text; text   @1 :Text; }   # dotted field name of the module, value as text

# One record as the service shows it to the asking person.
struct RecordView {
  id     @0 :Text;
  may    @1 :List(Text);                         # commands (own `commands.json` of the module) this person may run on it
  hidden @2 :List(HiddenField);
  values @3 :List(Value);
}

# What a write made or changed (appended with `invoke` in SW-1): the record it points at and a short text for people (a code, never a foreign sentence).
# Both empty = the command made or changed no record (for example a delete).
struct WriteResult { record @0 :Text; text @1 :Text; }

struct RecordPage {
  records        @0 :List(RecordView);
  total          @1 :UInt64;                     # exact total; read `hasTotal`
  hasTotal       @2 :Bool;
  totalEstimated @3 :UInt64;                     # estimated total; read `hasEstimate`
  hasEstimate    @4 :Bool;
  may            @5 :List(Text);                 # commands the asking person may run on the SET (not on one record), e.g. `task.create`: whether a "new card" control is open
}

# Pattern: Remote Proxy – der Host spricht mit dem Moduldienst über diese Capability wie mit einem Objekt; das Modul bleibt ein eigener Prozess.
# Abweichung: nicht durchsichtig – jede Antwort ist fremde Daten (eigene Grenzen, Steuerzeichen maskieren), Fehler sind sichtbar, kein stilles Wiederholen.
#
# Form key: ONE spelling, `name@x.y.z` (SemVer 2.0.0), everywhere (forms.txt, this wire, forms.json, layouts, messages). A reader also accepts the
# short form `x.y` as `x.y.0` (tolerant reader; `fs_interfaces::forms::parse_key`); a writer sends `x.y.z` (`forms::key`). A service compares
# name + major + minor; the patch does not count (it is never raised for forms).
#
# Errors (Level 1, fixed):
#   `hello`/`page`/`blob`: a failure is an RPC exception whose text carries the `Common.Problem` (fs-rpc `problem/wire.rs`): a read has no partial answer.
#   `invoke`: every domain refusal is in the ANSWER (`failed = true` + `problem`; `failed = false` means `problem` is empty). An RPC exception
#   at `invoke` means only "line or service broken" (reconnect), so a caller can tell a refusal (tell the person) from an outage.
#   Problem type = short relative URI reference `form-source-<kind>`:
#     method  kind              status
#     page    unknown-form      404
#     page    unknown-scope     404 (409: name not unique)
#     page    bad-arguments     400   (cursor)
#     blob    unsupported       501
#     blob    unknown-record    404
#     invoke  unknown-command   404
#     invoke  forbidden         403
#     invoke  bad-arguments     400
#     invoke  unknown-record    404
#     invoke  rejected          422
#     invoke  conflict          409
#     invoke  internal          500
interface FormSource {
  hello @0 () -> (level :UInt32, provider :Text);
  # The level of this contract the service speaks (a plain number, 1 today) and who it is (`fake` for example data).
  # A caller refuses a level it does not know, visibly.

  page @1 (form :Text, scope :Text, cursor :Common.Cursor, limit :UInt32) -> (page :RecordPage, next :Common.Cursor);
  # Records for a form (`name@x.y.z` of `forms.txt`), values by the field names of the module. `scope` narrows them (a board, a group, a
  # time range; its meaning belongs to the module, empty = all the person may see). `next` empty = the end. Unknown form or scope:
  # the call fails with a `Common.Problem` (`form-source-unknown-form`, `form-source-unknown-scope`).

  blob @2 (record :Text, field :Text) -> (blob :Common.Blob);
  # The bytes of an image or file field of one record (screenshots of capture).

  invoke @3 (command :Text, record :Text, args :List(Value)) -> (failed :Bool, problem :Common.Problem, result :WriteResult);
  # Writing, for example "move a card".
  # `args` carry the FIELD NAMES of the module (its `forms.json` map), never channel names (`lane`/`order`): the bridge channel -> field is the
  # `map` of the module, translated once by the caller (fs-forms-run `args_of`). The service checks them strictly against the `params` of the
  # command: unknown or doubled argument, or a missing required one = `form-source-bad-arguments`. `command` is the contract key (`card.move`),
  # not the host id (`planning.card.move`). Only commands of the module's own `commands.json` are allowed; the service checks the rights.
  # `failed = false`: done, `problem` empty.
  # `result` (appended): the record the command made or changed (`record`, empty for a delete) and a short text. A caller that creates a card and
  # moves it at once, or a workflow step that hands on the new record, reads it from here; an old service leaves it empty.
}
