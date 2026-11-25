//! The generator against the real schemas in `schema/`: what must always hold when someone edits a
//! contract. Pattern: none (tests).

mod support;

use fs_mcp::{Blocklist, MarkFiles, RiskClass, Target, Why};
use support::real::{contracts, generate_with, marks, real_catalog};

#[test]
fn every_method_and_union_member_has_a_mark() {
    let c = real_catalog();
    assert!(
        c.unmarked.is_empty(),
        "no entry in crates/fs-mcp/marks/*.toml for: {:?}",
        c.unmarked
    );
}

#[test]
fn no_mark_or_release_points_at_nothing() {
    let c = real_catalog();
    assert!(
        c.stale.is_empty(),
        "stale entries in the mark files: {:?}",
        c.stale
    );
}

#[test]
fn the_hard_exclusions_are_not_generated() {
    let c = real_catalog();
    for blocked in Blocklist::standard().targets() {
        assert!(
            c.tools.iter().all(|t| t.source != blocked),
            "{blocked} became a tool"
        );
        assert!(
            c.resources.iter().all(|r| r.source != blocked),
            "{blocked} became a resource"
        );
    }
    for t in &c.tools {
        assert!(
            !t.name.contains("decide") || !t.name.starts_with("messenger."),
            "{}",
            t.name
        );
        assert!(
            !t.name.starts_with("messenger.guard") && !t.name.contains("toolcall."),
            "{}",
            t.name
        );
        assert_ne!(t.risk, RiskClass::Excluded, "{}", t.name);
    }
    let left = |t: Target| c.left_out.iter().any(|l| l.target == t);
    assert!(left(Target::item("messenger", "MessageAction", "decide")));
    assert!(left(Target::item("messenger", "MessageAction", "none")));
    assert!(left(Target::owner("messenger", "Accounts")));
}

#[test]
fn the_messenger_act_tools_are_one_per_member() {
    let c = real_catalog();
    let mut acts: Vec<_> = c
        .tools
        .iter()
        .filter(|t| t.name.starts_with("messenger.conversation.act."))
        .map(|t| t.name.as_str())
        .collect();
    acts.sort_unstable();
    let expected = ["delete", "edit", "flag", "forward", "react", "unread"]
        .map(|m| format!("messenger.conversation.act.{m}"));
    assert_eq!(
        acts,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    let risk = |m: &str| c.tools.iter().find(|t| t.name.ends_with(m)).unwrap().risk;
    assert_eq!(risk("act.delete"), RiskClass::Destructive);
    assert_eq!(risk("act.forward"), RiskClass::ActsInName);
    assert_eq!(
        c.tools
            .iter()
            .find(|t| t.name == "message.send")
            .unwrap()
            .risk,
        RiskClass::ActsInName
    );
}

#[test]
fn without_any_marks_everything_is_unknown_and_the_exclusions_still_hold() {
    let none = MarkFiles::from_sources(&[]).unwrap();
    let c = generate_with(&contracts(), &none, Blocklist::standard());
    assert_ne!(c.tools, [] as [fs_mcp::Tool; 0]);
    assert!(
        c.tools
            .iter()
            .all(|t| t.risk == RiskClass::Unknown && t.unmarked && t.risk.asks_always())
    );
    assert!(
        c.tools
            .iter()
            .all(|t| !t.name.contains("act.decide") && !t.name.contains("act.none"))
    );
}

#[test]
fn without_the_block_list_the_secret_check_still_keeps_the_secret_out() {
    let c = generate_with(&contracts(), &marks(), Blocklist::empty());
    assert!(c.tools.iter().all(|t| !t.name.contains("accounts.add")));
    assert!(
        c.tools
            .iter()
            .all(|t| t.source != Target::item("surveys", "Credential", "token"))
    );
    assert!(
        c.left_out
            .iter()
            .any(|l| l.target == Target::owner("messenger", "Accounts")
                && matches!(l.why, Why::Marked(_)))
    );
    // and with no marks at all, the secret scan alone keeps `Accounts.add` out
    let bare = generate_with(
        &contracts(),
        &MarkFiles::from_sources(&[]).unwrap(),
        Blocklist::empty(),
    );
    assert!(
        bare.left_out
            .iter()
            .any(|l| l.target == Target::item("messenger", "Accounts", "add")
                && matches!(l.why, Why::SecretSuspect { .. }))
    );
}

const STORE_WRITES: [&str; 6] = [
    "install",
    "update",
    "uninstall",
    "addSource",
    "removeSource",
    "setEnabled",
];

fn store_marks(text: &str) -> MarkFiles {
    // all real marks, with the `store` file replaced
    let mut sources: Vec<(String, String)> = std::fs::read_dir(support::real::marks_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| {
            (
                p.file_stem().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    sources.retain(|(f, _)| f != "store");
    sources.push(("store".to_owned(), text.to_owned()));
    MarkFiles::from_sources(
        &sources
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn the_store_write_commands_are_excluded_a_person_only() {
    let c = real_catalog();
    for m in STORE_WRITES {
        let t = Target::item("store", "Store", m);
        assert!(c.tools.iter().all(|x| x.source != t), "{m} is a tool");
        assert!(
            c.left_out.iter().any(
                |l| l.target == t && matches!(&l.why, Why::Marked(r) if r.contains("ADR 0054"))
            ),
            "{m} is not left out as marked"
        );
        assert!(!c.unmarked.contains(&t), "{m} has no mark");
    }
    // `refresh`, `capabilities` and `job` stay tools (the other reads are held back by the secret check, not by a mark)
    for m in ["capabilities", "job", "refresh"] {
        let t = Target::item("store", "Store", m);
        assert!(c.tools.iter().any(|x| x.source == t), "{m} is gone");
    }
}

#[test]
fn counter_proof_a_store_write_command_with_an_old_class_or_no_mark_shows() {
    let real = std::fs::read_to_string(support::real::marks_dir().join("store.toml")).unwrap();
    for m in STORE_WRITES {
        let t = Target::item("store", "Store", m);
        // marked again as before: it becomes a tool
        let old = real
            .lines()
            .map(|l| {
                if l.starts_with(&format!("{m} = ")) {
                    format!("{m} = \"acts-in-name\"")
                } else {
                    l.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let c = generate_with(&contracts(), &store_marks(&old), Blocklist::standard());
        assert!(
            c.tools.iter().any(|x| x.source == t),
            "{m}: mutant not seen"
        );
        // no mark at all: it is listed as unmarked
        let none = real
            .lines()
            .filter(|l| !l.starts_with(&format!("{m} = ")))
            .collect::<Vec<_>>()
            .join("\n");
        let c = generate_with(&contracts(), &store_marks(&none), Blocklist::standard());
        assert!(c.unmarked.contains(&t), "{m}: missing mark not listed");
    }
}
