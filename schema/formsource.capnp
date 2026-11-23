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
struct Value       { path @0 :Text; text   @1 :Text; }   # dotted field name of the module, value as text

# One record as the service shows it to the asking person.
struct RecordView {
  id     @0 :Text;
  may    @1 :List(Text);                         # commands (own `commands.json` of the module) this person may run on it
  hidden @2 :List(HiddenField);
  values @3 :List(Value);
}

struct RecordPage {
  records        @0 :List(RecordView);
  total          @1 :UInt64;                     # exact total; read `hasTotal`
  hasTotal       @2 :Bool;
  totalEstimated @3 :UInt64;                     # estimated total; read `hasEstimate`
  hasEstimate    @4 :Bool;
}

interface FormSource {
  hello @0 () -> (level :UInt32, provider :Text);
  # The level of this contract the service speaks (a plain number, 1 today) and who it is (`fake` for example data).
  # A caller refuses a level it does not know, visibly.

  page @1 (form :Text, scope :Text, cursor :Common.Cursor, limit :UInt32) -> (page :RecordPage, next :Common.Cursor);
  # Records for a form (`name@H.N` of `forms.txt`), values by the field names of the module. `scope` narrows them (a board, a group, a
  # time range; its meaning belongs to the module, empty = all the person may see). `next` empty = the end. Unknown form or scope:
  # the call fails with a `Common.Problem` (`form-source-unknown-form`, `form-source-unknown-scope`).

  blob @2 (record :Text, field :Text) -> (blob :Common.Blob);
  # The bytes of an image or file field of one record (screenshots of capture).

  invoke @3 (command :Text, record :Text, args :List(Value)) -> (failed :Bool, problem :Common.Problem);
  # Writing, for example "move a card". Only commands of the module's own `commands.json` are allowed; the service checks the rights.
  # `failed = false`: done, `problem` empty.
}
