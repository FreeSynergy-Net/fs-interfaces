//! The contract of `LocalOwner`: the fakes pass, broken sources fail (counter-proof).
//!
//! Pattern: none, neutral - plain tests.
#![cfg(feature = "fake")]

use std::sync::atomic::{AtomicU8, Ordering};

use fs_interfaces::owner::fake::{FixedOwner, NoOwner, check, person, sample};
use fs_interfaces::owner::{LocalOwner, OwnerProblem};
use fs_types::{PersonArt, PersonRef};

#[test]
fn fixed_owner_passes() {
    assert_eq!(check(&FixedOwner(sample())), Ok(()));
}

#[test]
fn problems_pass_with_reason_and_have_codes() {
    let all = [
        (
            OwnerProblem::NoMe {
                hint: "create me".into(),
            },
            "owner-no-me",
        ),
        (
            OwnerProblem::Unreachable {
                reason: "no socket".into(),
            },
            "owner-unreachable",
        ),
        (
            OwnerProblem::Invalid {
                reason: "not v7".into(),
            },
            "owner-invalid",
        ),
    ];
    for (p, code) in all {
        assert_eq!(p.code(), code);
        assert!(p.to_string().starts_with(code));
        assert_eq!(check(&NoOwner(p)), Ok(()));
    }
}

#[test]
fn counter_proof_v4_fails_rule_1() {
    let v4 = person("urn:uuid:550e8400-e29b-41d4-a716-446655440000");
    assert!(check(&FixedOwner(v4)).unwrap_err().contains("not a UUIDv7"));
}

#[test]
fn counter_proof_other_breaks() {
    assert!(
        check(&FixedOwner(person("https://example.org/me")))
            .unwrap_err()
            .contains("urn:uuid")
    );
    let group = PersonRef {
        art: PersonArt::Group,
        ..sample()
    };
    assert!(
        check(&FixedOwner(group))
            .unwrap_err()
            .contains("not Person")
    );
    let bad_variant = person("urn:uuid:018f3a52-7c1e-7b3a-cd4e-5a6b7c8d9e0f");
    assert!(check(&FixedOwner(bad_variant)).is_err());
    let empty = OwnerProblem::NoMe { hint: " ".into() };
    assert!(check(&NoOwner(empty)).unwrap_err().contains("rule 3"));
}

#[test]
fn counter_proof_changing_answer_fails_rule_2() {
    struct Flip(AtomicU8);
    impl LocalOwner for Flip {
        fn owner(&self) -> Result<PersonRef, OwnerProblem> {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            Ok(person(if n == 0 {
                "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e0f"
            } else {
                "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e10"
            }))
        }
    }
    assert!(
        check(&Flip(AtomicU8::new(0)))
            .unwrap_err()
            .contains("rule 2")
    );
}
