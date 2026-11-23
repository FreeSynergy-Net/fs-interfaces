//! Example of `groups`: the same contract in two shapes. A network built from below (a local group is the seed,
//! levels are edges above it, a group of organisations only advises on a `Single` edge) and a company built from
//! above (board, departments, teams). Invented example data (`Beispieldaten`), nothing real.
//!
//! Pattern: Fake behind the traits (the example talks to `FakeGroups` through `Groups` and `GroupGovernance` only).
//!
//! Run: `cargo run -p fs-group-directory --example two_trees`.

use fs_group_directory::groups::{GroupGovernance, Groups};
use fs_group_directory::{EXAMPLE_LOCAL_MEMBERS, example};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let e = example(EXAMPLE_LOCAL_MEMBERS);
    println!("build problems: {}", e.problems().len());
    for (name, who) in [
        ("Weltnetz Lindenau", "from below: members rise to the top"),
        (
            "Team Testautomatisierung",
            "from above: the board reaches the team",
        ),
        (
            "Organisationsgruppe Wasserwerk",
            "advisory (single edge): nothing flows",
        ),
    ] {
        let g = e.id(name).expect("group");
        let n = e
            .groups
            .effective_members(g, None, 10_000)
            .await
            .expect("members")
            .items
            .len();
        let o = e.groups.owners(g).await.expect("owners").len();
        println!("{name}: {n} effective members, {o} owner ({who})");
    }
}
