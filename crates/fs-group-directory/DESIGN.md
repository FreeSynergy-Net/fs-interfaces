# fs-groups-testdata (design)

Step K0 of the pilot `programm-grundgeruest` for groups. The crate holds everything a frontend or a provider needs
to work against the locked trait `groups_v1` before the real provider (`LocalGroups`, step K1) exists:

| Part | What it is |
|---|---|
| `groups_v1/` | The LOCKED trait folder, byte for byte (SHA-256 `91c0f8b0849d85b1038b8b09f511e27134c1ddfeb0ef2eae42dc8579b3d2faf3`, lock file `.scratch/sperren/groups_v1.sha256`). Never edited; a change is `groups_v2`. `lib.rs` carries the allows the 2024 workspace needs and `#[rustfmt::skip]`, outside the folder. |
| `FakeGroups` | In memory, implements `Groups`, `Mirroring` and `GroupMembers` completely. |
| `FakePeople`, `FakeMirror` | The consumed sides: fs-people (`PersonDirectory`, `PersonFacts`, `RoleBook`) and Kanidm (`GroupDirectory`). |
| `FailingGroups`, `SlowGroups` | Decorators over the fake (or any provider of the three traits): fail every method, chosen methods (`Op`) or every call that names a chosen group, `heal()`; or wait on the fake's clock (virtual, nobody sleeps). |
| `sample()` | The fixed data set (PLAN section 6): 16 groups (one in the trash) and 20 invented persons on `example.org` (2 AI accounts, 1 unknown): a BottomUp tree, a TopDown chain of nine levels, a group with several parents (BottomUp and TopDown), a Single branch, an organisation, a sensitive, an admin and an AI group, two mirrored, one open edge request and one open mode-change request, invitations pending, accepted, declined and expired, rights through a holder group. |
| `conformance` | `run_conformance(&dyn ConformanceFixture)`: the rules every provider must pass. |

Pins: `fs-types` v0.0.22 (= fs-interfaces `7743056396ffa80bc77d3af9d1946229dd1f6155`) as git tag, `async-trait`. Nothing
else. The workspace version is the candidate of ADR 0064 (no tag); `fs-release` pins are untouched.

## The fake

`FakeGroups` is a `Mutex<State>`; the trait impl (`fake_impl.rs`) only takes the lock and calls a sync operation on
the state. The operations are split by theme (`ops_*.rs`); `access.rs` answers who may do and see what; `nest_check.rs`
is the one check every writer of edges calls (request, answer, mode change, `restore`).

* **Who asks** is a property of the provider (`set_asker`), never a parameter (trait rule). The clock is the
  provider's too; every write moves it one second; the trash deadline is 30 days (`set_now`).
* **The AI visibility rule** is `State::visible`: unknown, deleted or hidden is the same `NotFound`, and every method
  with a group id calls it first. Hidden = the asker is marked as AI and the group is sensitive or carries an admin
  role. Numbers and filters (`parents`, `children`, `Roots`, `ChildrenOf`), paths (`ancestors`, `Cycle`), grants,
  history, changes and the origin of effective members leave hidden groups out (`from = None`, `via` empty).
* **`InForce`** is computed when read, never stored: the account kind fits the group or it does not (`KindMismatch`),
  an unknown person with an admin role or `Manage` is `AiStatusUnknown`. A mark that changes after the fact needs no
  write; the membership stays and stops counting.
* **Mirror**: the plan is a pure computation on the state against the directory as it is (`mirror_plan.rs`), the run
  (`mirror_run.rs`) writes step by step; the ticket is accepted only when a fresh plan equals the shown one
  (`PlanStale` otherwise). By-hand things in the directory are findings, never silent steps.

`DECISIONS` (in `fake.rs`) lists what the fake decided where the trait is silent; the conformance rules do not test
those, only what the trait or the plan says.

## `nesting_logic` (the seam)

`nesting_logic/` is the PURE nesting rule over ids and edges (arcs of the member graph, cycle in the tree and in the
member graph, depth, the hull with the shortest way, reach for ancestors and descendants). It is the seed of the later
crate `group-nesting` (written in a parallel run) and is replaced by it: only `state.rs`, `bridge.rs`, `nest_check.rs`,
`access.rs`, `ops_tree.rs` and `mirror_plan.rs` call it, through its small surface (`Edge`, `Mode`, `arcs`,
`find_cycle`, `too_deep`, `hull`, `reach`). `bridge.rs` maps the locked `NestingMode` to the logic's `Mode`.

## Conformance

`run_conformance` takes a `ConformanceFixture`: the provider plus what the trait cannot do (make a human, an AI or an
unknown person, say who asks, move the clock, touch a deleted group, change the directory by hand). A mutator the
fixture lacks keeps its default; the rules that need it are reported in `Report::skipped`, never as passed. The rules
seed their own data (`Zzconf ...` groups) and run on an empty or a filled provider. `Report::checked` counts the
checks that ran (the tests assert a floor, so a rule that quietly cannot set up is noticed).

Rule files: groups (create, read, update, list), trash (`delete`, `restore`, Changed, deadline), nesting (modes,
rights flow, unequal depth, cycles, depth in the tree AND in the member graph (zigzag chain, TopDown chain), sensitivity, mode changes, `nesting_effect`), requests (both sides agree),
members (roles, visibility, leaving, `LastManager`, paging), invites (all states, expiry), rights (`grant`, `may_do`
with reasons, change stream, history), AI (kinds, edges, switch, `InForce`, admin roles, `restore`; an AI never agrees to a request on a visible group and never gets the numbers of `nesting_effect`), hidden (every
method with a hidden group, `NotFound` before any other error, the nine sub-object ids, the narrow view, the mirror,
the change stream), mirror (plan, ticket, findings, native nesting per mode, AI and admin rules on the hull).

`tests/conformance/` runs the rules against the fake (empty and on the sample) and against four mutants that each
break one rule (a hidden group leaking through `members`, `nesting_effect` answering `Forbidden`, a cycle called
`AlreadyThere`, no compare-and-set); the rules must fail for them. `tests/sample.rs` checks the sample's shape,
`tests/doubles.rs` the two decorators, `tests/cycles.rs` the cycle path and the depth check on old data with a cycle.

Mutants proven red in a copy (K0b, source mutations of the fake, one rule each): depth only in the tree (zigzag rule);
an AI agrees to a request without `Manage`; `nesting_effect` for an AI without the `Manage` check (numbers leak); the
old recursive `longest` (stack overflow on a cycle); cycle path in edge order (the order test). Not provable: dropping
the `!hidden` filter inside `nesting_effect` changes nothing visible, because an asking AI never gets `Manage` and so
never reaches the counting, and a human sees every group; the filter is a guard, the rule is tested at the door.

## For K1 (the move into the core)

The trait folder lives here (`src/groups_v1/`) so the frontends can start. `LocalGroups` in `fs-groups-core` cannot
depend on this crate, so at step K1 the folder moves BYTE FOR BYTE to `fs-groups-core/src/interface/groups_v1/`
(check with `hash-ordner.sh`: the hash above before and after) and this crate re-exports it (`pub use
fs_groups_core::interface::groups_v1`). The lines of the `mod` declaration (lock comment, `#[rustfmt::skip]`, allows)
move with it. The fake's `nesting_logic` is replaced by the crate `group-nesting` at the same time (see below).

## Known deviations from `group-nesting` (decided at the switch)

The fake's pure logic and the parallel crate `group-nesting` agree on: turning `TopDown`, the cycle (tree, then member
graph), depth 16/17, sensitive in both directions, AI parents and children, `AlreadyThere` and mode change without an
old edge, the hull backwards without `Single` and old edges. They differ at the edges, and the switch must settle each
(the fake's answer stays what `DECISIONS` says until then):

1. **A self edge at an AI group:** the fake says `Cycle [A, A]` (the plan: a self edge is a cycle), `group-nesting`
   says `AiNesting` (its AI check runs first). Decide at the switch which one the trait promises.
2. **Old edges at an AI group** (put in raw, never written by a provider): the fake treats them like `Single` in the
   hull AND in the cycle and depth checks (nothing flows, nothing is counted); `group-nesting` treats them like `Single`
   in the hull but counts them in the cycle and depth checks. The conformance rules do not reach old data.
3. **Cycle path:** of several equally short cycles the fake names the smallest in id order (tested); `group-nesting`
   names the first in its sorted order. The rules fix only `[A, A]` and the form (first group = last group).
4. **`longest`** is safe on a cycle in the fake (it does not follow the way that closes one; iterative, no stack
   overflow) like `group-nesting`'s; it does not report the cycle, the cycle check does.

## Not covered (honest list)

* `FakeMirror` has `set_down`; `FailingGroups` fails the three traits, but not the `GroupDirectory` the mirror runs against
  (that is `FakeMirror::set_down`).
* `FindingKind::Unreadable` and `Looped` of the mirror are never produced (the fake's data cannot be unreadable or
  looped; a real provider needs rules for them).
* Zufallsgraphen (random graphs against the pure logic) belong to `group-nesting`, not here.
* `restore` needs `Manage` and says `Changed` for a deleted group touched since the deletion: the trait says nothing
  about who may restore; this is a decision of the fake (`DECISIONS`).
