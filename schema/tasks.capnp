# fs-tasks contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-tasks. Lives in fs-interfaces (ADR 0045).
#
# Target rules (ADR 0045): the interface is a target, not a snapshot. It stays DRAFT until two
# independent consumers have tested it, then it becomes 1.0, and from then on only version jumps.
# Consumers so far: fs-calendars (1 of 2).
#
# Source: docs/en/programs/apps/tasks/design.md (interface sketch). The Rust form of `Items` and of
# the subscription is the trait pair `TaskItems` / `TaskFeed` in crates/fs-tasks-core (ADR 0043:
# trait first). A task travels as its JSCalendar 2.0 JSON text, which stays authoritative
# (the standards are JSON; ADR 0014, 0050), next to the AS2 envelope.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.
# Compiled with common.capnp by capnp-compiler (see crates/fs-interfaces/crates/regenerate).

@0xc04d258971543476;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;
using T = import "/types.capnp";

# RFC 9253 relation of a task to another object (JSCalendar `relatedTo`)
enum Relation {
  first          @0;
  next           @1;
  child          @2;
  parent         @3;
  dependsOn      @4;
  finishToStart  @5;
  finishToFinish @6;
  startToStart   @7;
  startToFinish  @8;
}

# Whether a task can be started, from its dependencies (0.0.19; plan of fs-tasks, ADR 0073). Clients must tolerate values they
# do not know. `ReadinessReport.by` says which tasks the state is about.
enum Readiness {
  ready               @0;   # nothing blocks it
  blocked             @1;   # `by` = uids of the unfinished tasks it waits for
  done                @2;   # progress is completed
  unknown             @3;   # `by` = uids a dependency names that this store does not hold (shown, never guessed)
  dependencyCancelled @4;   # `by` = uids of dependencies that are cancelled or failed: a person decides, nothing frees it by itself
  conflict            @5;   # `by` = the uids of a dependency cycle the task is part of (detected on read, also after a merge)
}

struct ReadinessReport { readiness @0 :Readiness; by @1 :List(Text); }

struct Task {
  envelope   @0 :Envelope;        # AS2 `Task` envelope (id, iri, name, context "tasks", times)
  jscalendar @1 :Text;            # the JSCalendar 2.0 `Task` as JSON (authoritative content)
  revision   @2 :UInt64;          # changes at every write (for editIf); compare for equality only, it is no counter
}

# one page of tasks, ordered by due date (undated last)
struct TaskPage { items @0 :List(Task); next @1 :Cursor; }

# all members optional; unset (null / empty / false) = no restriction; combined with AND.
# The filter applies to `query` on every page: the cursor continues the same filtered, ordered list, so
# the caller sends the same filter with each cursor (`list` takes no filter and returns everything).
# "Open" = not finished: needs-action and in-process. completed, failed and cancelled are finished.
# "Done" is `progress = "completed"`; `open` and `progress` are both applied (AND), so `open = true`
# together with a finished `progress` matches nothing.
struct TaskFilter {
  dueFrom  @0 :Timestamp;         # due at or after
  dueTo    @1 :Timestamp;         # due before (half-open range)
  progress @2 :Text;              # needs-action | in-process | completed | failed | cancelled
  label    @3 :Text;              # JSCalendar keyword
  term     @4 :Text;              # substring of title and description; case-insensitive; empty = no restriction
  open     @5 :Bool;              # true = only open tasks: progress needs-action or in-process (a task without progress counts as needs-action); false = no restriction
}

struct ImportReport {
  created @0 :List(Uuid);
  skipped @1 :List(Text);         # uids that already existed
  failed  @2 :List(Text);
  notes   @3 :List(Text);
}

# root capability (capability ids: task.create, task.edit, task.assign, relation.create, label.assign,
# task.read, task.query, task.delete, task.import, task.export, task.subscribe, task.claim; boards: board.open, board.create,
# column.create, column.limit, card.move)
interface Tasks {
  capabilities @0 () -> (capabilities :CapabilityList);
  items        @1 () -> (items :Items);
  subscribe    @2 (listener :ChangeListener) -> (subscription :Subscription);
  boards       @3 () -> (boards :Boards);      # methods since 0.0.29, see below
  supportReport @4 () -> (report :C.SupportReport);  # provider rule (ADR 0045)
  # The data types this program owns (`Task`) and what can be done with them (ADR 0066), for a generated frontend.
  types         @5 () -> (catalog :T.TypeCatalog);
}

interface Items {
  create         @0  (jscalendar :Text) -> (task :Task);                                   # task.create
  edit           @1  (id :Uuid, jscalendar :Text) -> (task :Task);                         # task.edit
  assign         @2  (id :Uuid, name :Text, email :Text) -> (task :Task);                  # task.assign
  createRelation @3  (id :Uuid, otherUid :Text, relation :Relation) -> (task :Task);       # relation.create
  assignLabel    @4  (id :Uuid, label :Text) -> (task :Task);                              # label.assign
  get            @5  (id :Uuid) -> (found :Bool, task :Task);
  getByUid       @6  (uid :Text) -> (found :Bool, task :Task);
  delete         @7  (id :Uuid) -> ();
  list           @8  (cursor :Cursor) -> (page :TaskPage);
  query          @9  (filter :TaskFilter, cursor :Cursor) -> (page :TaskPage);             # time range, status, label
  importIcs      @10 (text :Text) -> (report :ImportReport);                               # VTODO
  exportIcs      @11 (ids :List(Uuid)) -> (text :Text);
  editIf         @12 (id :Uuid, revision :UInt64, jscalendar :Text) -> (task :Task);       # task.edit, only if the task still has `revision`; otherwise it fails with a reason starting `conflict:` and writes nothing (compare-and-set; atomic inside one provider process only)
  # Appended in 0.0.19. Capability ids `dependency.evaluate` (readiness, blockedBy, dependents) and `dependency.cycle-check`.
  readiness      @13 (id :Uuid) -> (report :ReadinessReport);                              # dependency.evaluate
  blockedBy      @14 (id :Uuid) -> (tasks :List(Task));                                    # dependency.evaluate; the unfinished tasks it waits for
  dependents     @15 (id :Uuid) -> (tasks :List(Task));                                    # dependency.evaluate; the tasks that wait for it (reverse edges)
  # Like `createRelation`, with the lag (positive) or lead (negative) of a temporal relation: an ISO 8601 duration such as
  # `P1D` or `-PT4H`, stored as the member `freesynergy.net:gap` of the JSCalendar `Relation` and written as RFC 9253 `GAP`.
  # Empty = none. A gap on a relation without temporal meaning is refused. `createRelation` stays as it was.
  createRelationWithGap @16 (id :Uuid, otherUid :Text, relation :Relation, gap :Text) -> (task :Task);   # relation.create
  # A relation that would close a dependency cycle is refused by `createRelation*`: the problem type is `cycle` (status 409),
  # its extension `path` repeats once for every uid of the cycle, in order.

  # Appended in SW-1. A claim holds a task for one person for a limited time (the 24-hour limit is the module's rule, enforced by the provider).
  # The person is always the caller of the connection: there is NO `by` parameter, nobody claims for somebody else. `until` is the end of the claim.
  # Both are compare-and-set on `revision` like `editIf`: a changed task or a foreign claim fails with a reason starting `conflict:`. Capability id `task.claim`.
  claim   @17 (id :Uuid, revision :UInt64, until :Timestamp) -> (task :Task);    # task.claim
  release @18 (id :Uuid, revision :UInt64) -> (task :Task);                       # task.claim (only the claimer lets go)
}

# Boards (methods appended in 0.0.29; the interface was empty before). The Rust form is the trait `Boards` in
# crates/fs-tasks-core. A board is its own object, never a member of a task (a task can be on several boards); a card is a
# task `uid`. Capability ids: board.open (list, layout), board.create (create), column.create and column.limit (editColumns),
# card.move (moveCard). Moving a card is an `Update` on `task_link`, never on the task (projects/technical.md); the progress
# of the task does not change by it (that is an edit of the task).

# One column of a board. `id` is stable inside the board.
struct BoardColumn {
  id       @0 :Text;
  name     @1 :Text;
  wipLimit @2 :UInt32;            # the most cards the column should hold; 0 = no limit. More is reported by the layout, never refused
  progress @3 :List(Text);        # the `progress` values whose tasks stand in this column unless they were placed elsewhere
}

# The placed cards of one column: the task `uid`s, top to bottom.
struct Placement { column @0 :Text; cards @1 :List(Text); }

struct Board {
  envelope @0 :Envelope;          # the board as an object (id, name, times)
  name     @1 :Text;
  label    @2 :Text;              # the tasks on it: those with this JSCalendar keyword; empty = every task (placed cards are on it too)
  columns  @3 :List(BoardColumn); # left to right
  order    @4 :List(Placement);   # only columns that hold placed cards
  revision @5 :UInt64;            # changes at every write (for editColumns); compare for equality only, it is no counter
}

# A column with the cards in it now: the placed ones first, then those whose progress puts them there.
struct ColumnLayout {
  column    @0 :BoardColumn;
  cards     @1 :List(Text);       # task `uid`s, top to bottom
  overLimit @2 :Bool;             # more cards than the limit says (false without a limit); a report, never a refusal
}

struct BoardLayout { columns @0 :List(ColumnLayout); }

interface Boards {
  create      @0 (name :Text, label :Text) -> (board :Board);                                                 # board.create; the standard columns (to do, in progress, done, closed); empty label = every task
  list        @1 () -> (boards :List(Board));                                                                 # board.open; by name
  layout      @2 (id :Uuid) -> (layout :BoardLayout);                                                         # board.open; the columns with their cards now
  # Replaces the columns (and so their limits) and the placed cards, only if the board still has `revision`; otherwise it fails
  # with a reason starting `conflict:` and writes nothing. A board that is not valid (no column, a column id empty or twice,
  # a limit that makes no sense, a card placed twice or in a column that does not exist) fails with a reason starting `rejected:`.
  editColumns @3 (id :Uuid, revision :UInt64, columns :List(BoardColumn), order :List(Placement)) -> (board :Board);   # column.create, column.limit
  # Places the card of the task `uid` in `column` at `index` (negative = at the end; larger than the column = at the end) and takes
  # it out of any other place. A full column does not refuse it. An unknown column fails with a reason starting `rejected:`.
  moveCard    @4 (id :Uuid, uid :Text, column :Text, index :Int32) -> (board :Board);                         # card.move
}
