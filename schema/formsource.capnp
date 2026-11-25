# DRAFT contract `FormSource` (owner fs-forms; PLAN-quelle-und-abgleich section 2, order SQ-1b). Every module service serves it next to its
# own contract, the way it serves `Control`: the host (fs-starter) finds the service, checks it and connects; fs-forms draws by the
# form (`schema/forms.txt`). The form names are NOT part of this file (they are data in `forms.txt`, read by `fs_interfaces::forms`).
# Handshake: `hello` answers a plain level number (`FORM_SOURCE_LEVEL` in `fs_interfaces::forms`), never a name like `_v2`. A change that
# breaks is not a new level (it raises the minimum or is a new contract); levels are cumulative and a higher one only appends methods and fields. Evolution (Cap'n Proto): only append, never renumber or remove.
# Answers are foreign data for the caller: it keeps its own limits (page size, bytes, time) and masks control characters before drawing.
# A service that serves example data says `provider = "fake"`; the window then shows "example data" and never passes it off as real.

@0xfa9420fd361f284b;

using Common = import "common.capnp";

enum HiddenReason {
  noRight   @0;                                  # the asking person has no right to the field
  sensitive @1;                                  # the field is sensitive
}

struct HiddenField { path @0 :Text; reason @1 :HiddenReason; }
struct Value       { path @0 :Text; text   @1 :Text; }   # dotted field name of the module, value as text

# One record as the service shows it to the asking person.
struct RecordView {
  id     @0 :Text;
  may    @1 :List(Text);                         # commands (own `commands.json` of the module) this person may run on it
  hidden @2 :List(HiddenField);
  values @3 :List(Value);  # one `Value` per field; a channel marked `repeats` in forms.txt may repeat its `path`, once per item (rule in `FormSource`)
}

# What a write made or changed (appended with `invoke` in SW-1): the record it points at and a short text for people (a code, never a foreign sentence).
# Both empty = the command made or changed no record (for example a delete).
struct WriteResult { record @0 :Text; text @1 :Text; change @2 :ChangeRef; }   # @2 appended (level 2); an old service leaves it empty

struct RecordPage {
  records        @0 :List(RecordView);
  total          @1 :UInt64;                     # exact total; read `hasTotal`
  hasTotal       @2 :Bool;
  totalEstimated @3 :UInt64;                     # estimated total; read `hasEstimate`
  hasEstimate    @4 :Bool;
  may            @5 :List(Text);                 # commands the asking person may run on the SET (not on one record), e.g. `task.create`: whether a "new card" control is open
}

# Stufe 3 (level 2), appended: who changed what, the compare-and-set conflict, undo and the push of changes.
# Pattern: Value Object - where a change came from (given by `introduce`, never a right) and who made it (from the connection).
struct Origin {
  person   @0 :Text;   # IRI of the person (PersonRef.id), from the connection of the module; never from the caller
  program  @1 :Text;   # AppStream id, given (introduce); empty allowed
  instance @2 :Text;   # instance of the program (host process/window), given; empty allowed
  device   @3 :Text;   # device key (later the iroh NodeId), never a display name; empty allowed
}

# Pattern: Idempotent Receiver - the same non-empty key acts once; the repeat gets the same answer.
# Deviation: an opaque text of the run; the module only compares for equality.
struct StepRef { run @0 :Text; step @1 :Text; }          # both empty = no run

# Pattern: Memento (reference) - the module holds the data, the caller only the reference.
struct ChangeRef {
  activity @0 :Common.Uuid;   # head activity of the unit (UUIDv7) = id of the change
  object   @1 :Text;          # record id (Envelope.id, UUIDv7 as text)
  revision @2 :Text;          # objects.rev after the change (decimal text)
  before   @3 :Text;          # objects.rev before; empty at create
  origin   @4 :Origin;
  kind     @5 :Text;          # AS2: Create | Update | Delete | Undo
  at       @6 :Common.Timestamp;
  undoes   @7 :Common.Uuid;   # at kind = Undo: the change taken back; else 0
}

# Pattern: Value Object - one field of the conflict: what the record has now and what the caller wanted.
struct FieldClash { path @0 :Text; current @1 :Text; mine @2 :Text; }

# Pattern: Value Object - the conflict, typed, in the answer.
# Deviation: not a problem extension. `Common.Extension` carries only text pairs; a list of triples would be an invented encoding.
# The problem still has type `form-source-conflict`, 409 and `issue=changed-since`; the details are here.
struct Conflict { by @0 :ChangeRef; fields @1 :List(FieldClash); }

# Pattern: Observer - receiver of the changes of a form; rules as `Common.ChangeListener` (stream, `ended`, never answer with an error).
# Deviation: an own receiver with `ChangeRef` instead of `Common.Change`/Envelope: the records of FormSource are field views, not
# envelopes, and `changes` (query) and `watch` (push) deliver the same element this way.
interface ChangeSink {
  changed @0 (change :ChangeRef) -> stream;
  ended   @1 (problem :Common.Problem) -> ();
}

interface FormSource {
  # Pattern: Remote Proxy - the host talks to the module service through this capability as to an object; the module stays its own process.
  # Deviation: not transparent - every answer is foreign data (own limits, mask control characters), errors are visible, no silent retry.
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
  #
  #   Three rules that were gaps (stated here, no wire change):
  #   - `limit = 0` means "the service's own default page size" (never an empty page for a non-empty set); a larger `limit` may be capped by the
  #     service to its own maximum, and `next` then says whether more follows.
  #   - An empty page (`records` empty) always has an empty `next`; otherwise a caller pages forever.
  #   - A refused write (`failed = true`) leaves `result` empty: it names no record and carries no text.
  #
  #   Values: a value of art `time` (in `page` records, also a `facts` field of art time, and in `invoke` args; for example `due`) is an RFC 3339
  #   `date-time` (section 5.6) WITH an offset or `Z`, never a local time without offset. Channel `progress` is a percentage 0..100.
  #   Repeated values (cards-in-columns@1.4.0 `assignee`, single-record@1.2.0 `fields`; text only): a channel that the catalog comment marks `repeats` has one `Value` per item, all with the same `path`, in the order of the module; every other channel has at most one.

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
  # Pattern: Command - a write command is a value (contract key, record, arguments by field name) that the module checks and runs.
  # Deviation: level 1 without id, expectation and undo; those come appended with level 2 (invokeIf/revert/StepRef).
  # Position (S1 E3): a place in an ordered set is NEVER an integer index. A move names its neighbours by record id (`after` = the record it
  # comes right after, `before` = the record it comes right before); a fractional key is made only by the module, never by the caller.
  # `args` carry the FIELD NAMES of the module (its `forms.json` map), never channel names (`lane`/`order`): the bridge channel -> field is the
  # `map` of the module, translated once by the caller (fs-forms-run `args_of`). The service checks them strictly against the `params` of the
  # command: unknown or doubled argument, or a missing required one = `form-source-bad-arguments`. `command` is the contract key (`card.move`),
  # not the host id (`planning.card.move`). Only commands of the module's own `commands.json` are allowed; the service checks the rights.
  # At a create of `cards-in-columns`, the field the module maps from channel `lane` is optional: if it is missing and the module has exactly ONE
  # lane (for example one board), that one is meant; none or several = `form-source-bad-arguments` 400 with a text for people, never a silent guess.
  # `failed = false`: done, `problem` empty.
  # `result` (appended): the record the command made or changed (`record`, empty for a delete) and a short text. A caller that creates a card and
  # moves it at once, or a workflow step that hands on the new record, reads it from here; an old service leaves it empty.

  # ---- Level 2 (Stufe 3), appended: @4 to @8. A service of level 1 answers these with `unimplemented`; the host asks `hello` first. ----

  invokeIf @4 (command :Text, record :Text, args :List(Value), expect :Text, step :StepRef)
           -> (failed :Bool, problem :Common.Problem, result :WriteResult, conflict :Conflict);
  # Pattern: Command - a write command with an expectation (compare-and-set) and a step; the answer names the change.
  # expect = objects.rev of the record as decimal text; empty = any. If it does not match: failed, form-source-conflict 409, `conflict` filled.
  # The same non-empty step: no second effect, the same answer (same ChangeRef), also after a restart of the module.

  revert @5 (change :Common.Uuid, step :StepRef)
         -> (failed :Bool, problem :Common.Problem, result :WriteResult, conflict :Conflict);
  # Pattern: Compensating Transaction (Memento) - writes the before-images of the unit as a new `Undo` unit. Repeating = revert of the Undo.
  # Deviation: compare-and-set; it fails visibly: form-source-conflict 409 (+conflict.by), form-source-not-revertible 422, form-source-gone 410.

  changes @6 (scope :Text, cursor :Common.Cursor, run :Text, limit :UInt32) -> (items :List(ChangeRef), next :Common.Cursor);
  # Pattern: Observer (query). run empty = the changes of the own program (origin.program from introduce), newest first;
  # run set = only those of this run (to settle after a crash). scope as at `page`.

  introduce @7 (program :Text, instance :Text, device :Text) -> ();
  # Pattern: Context Object (login) - holds for THIS connection until it ends; "given", counts for history/undo, never for rights.
  # Twice on one connection: the second value holds. Without introduce: origin.program/instance are empty.

  watch @8 (form :Text, scope :Text, sink :ChangeSink) -> (subscription :Common.Subscription);
  # Pattern: Observer - every change in the scope, by WHOMEVER (other programs too), after the commit. Dropping the subscription ends it.
  #
  # Problem kinds appended to the table above (level 2):
  #   method    kind              status
  #   invokeIf  conflict          409
  #   revert    conflict          409
  #   revert    not-revertible    422
  #   revert    gone              410
  #   revert    unknown-change    404
  #   changes   bad-arguments     400
  #   watch     unknown-form      404   (watch throws an RPC exception like `page`)
}
