# fs-settings contract — DRAFT 0.1 (2026-11-17)
#
# STATUS: DRAFT. Owner program: fs-settings (library `fs-settings`, programs `fs-settings-bin/-server/-app`). Lives in
# fs-interfaces (ADR 0045, 0100). PROVISIONAL: replaced by the storage trait draft (trait round, fs-settings-store) when it lands.
# Consumers so far: none (0 of 2).
#
# Source: .scratch/katalog-plan/trait-runde/fs-settings/settings.capnp and PLAN-speicher-schnittstelle.md. The Rust form is the
# trait `SettingsStore` in the crate fs-settings (Rust traits live there, this file is only the wire form).
#
# Rules: typed, no JSON (ADR 0086). A failure is a `Common.Problem` whose `type` is the URI
# "https://freesynergy.net/problems/settings/<code>" and whose `extensions` carry the parameters (key/value pairs); `title` and
# `detail` hold the code and nothing else, never an English sentence (ADR 0057). Codes: store-broken (file, cause), store-newer
# (file, found, known), store-unreachable (socket), store-busy (file), read-only (which), locked (key), not-declared (key),
# wrong-type (key), reason-missing (key), not-allowed (key), no-such-profile (name), profile-exists (name), import-invalid
# (file, line, cause), import-conflict (key, cause). Times are UTC (`Common.Timestamp`, milliseconds).
# Caller identity: the server takes it from the connection (the socket peer, the session), never from a parameter; what a
# caller says about itself is not believed. `lockedBy` / `by` in answers are set by the server.
# Place: where a value is kept (profile of the person, or this device); the device is a place, not a layer (ADR 0100).
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.
# Compiled with common.capnp by capnp-compiler (see crates/regenerate).

@0x962084020d866277;

using C = import "common.capnp";
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Problem = C.Problem;
using Subscription = C.Subscription;

# What a store keeps for one value (matches the `BasicType` kinds flag, whole, text of fs-types 0.0.3).
struct Raw {
  union {
    flag  @0 :Bool;
    whole @1 :Int64;
    text  @2 :Text;
  }
}

# The layer a value comes from, lowest first; the effective value is the highest layer that has one.
enum Layer { default @0; instance @1; system @2; person @3; program @4; namedPerson @5; namedProgram @6; }
enum Place { profile @0; device @1; }
enum Target { thisProgram @0; allPrograms @1; }

struct Kind {
  union {
    flag     @0 :Void;
    whole    @1 :Range;
    choice   @2 :List(Text);
    language @3 :Void;
    text     @4 :UInt32;                                  # maximum characters
  }
  struct Range { min @0 :Int64; max @1 :Int64; }
}

# One declared setting; label and description are message keys derived from `key`, not texts.
struct Declaration {
  key     @0 :Text;
  kind    @1 :Kind;
  default @2 :Raw;
  group   @3 :Text;
  place   @4 :Place;
}

struct Lock {
  reason   @0 :Text;                                      # required
  lockedBy @1 :Text;                                      # IRI of the actor, set by the server
  lockedAt @2 :Timestamp;
}

struct Effective {
  key     @0 :Text;
  value   @1 :Raw;
  from    @2 :Layer;
  lock    @3 :Lock;                                       # meaningful only if `locked`
  locked  @4 :Bool;
  canLock @5 :Bool;                                       # the caller may lock (admin right from fs-access)
}

# Is the store usable? `degraded` = emergency mode: built-in defaults, writes refused with `read-only`; the shell shows it.
struct Health {
  degraded @0 :Bool;
  problems @1 :List(Problem);
}

struct LogEntry {                                         # instance log, append only
  id     @0 :Data;                                        # UUIDv7
  at     @1 :Timestamp;
  by     @2 :Text;
  action @3 :Action;
  key    @4 :Text;
  reason @5 :Text;
  oldValue @6 :Raw;
  newValue @7 :Raw;
  enum Action { set @0; clear @1; lock @2; unlock @3; }
}

interface Settings {                                      # one per program and person; rights come from fs-access
  catalog @0 () -> (all :List(Declaration));
  get     @1 (keys :List(Text)) -> (values :List(Effective));
  # An empty `keys` list is not "all": use `list`.
  list    @2 (group :Text, cursor :Cursor) -> (values :List(Effective), next :Cursor);
  set     @3 (key :Text, value :Raw, reset :Bool, target :Target) -> ();   # `reset` removes the person's value
  watch   @4 (listener :Listener) -> (subscription :Subscription);
  health  @5 () -> (health :Health);
  # Named profiles: the person's own, never another person's; the active one is per device.
  profiles      @6 () -> (names :List(Text), active :Text);       # active "" = the base ("Allgemein")
  createProfile @7 (name :Text) -> ();
  switchProfile @8 (name :Text) -> ();                            # "" = the base
  renameProfile @9 (old :Text, new :Text) -> ();
  deleteProfile @10 (name :Text) -> ();                           # the id is never reused
  interface Listener { changed @0 (change :Effective) -> stream; }
}

interface Instance {                                      # the admin's side; every call checks the admin right
  instanceSet @0 (key :Text, value :Raw, reset :Bool) -> ();      # instance default
  lock        @1 (key :Text, value :Raw, reason :Text) -> ();     # lockedBy/lockedAt set by the server
  unlock      @2 (key :Text, reason :Text) -> ();
  log         @3 (after :Data, size :UInt32) -> (entries :List(LogEntry));   # read only
}
