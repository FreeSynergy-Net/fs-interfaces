//! The first people of the group sample are the people of the directory sample (one source of names).
//!
//! Pattern: none (tests). Red when a person goes back to an invented reference or name.

use fs_group_directory::groups::PersonDirectory;
use fs_group_directory::sample;
use fs_person_directory::{sample_name, sample_ref};

#[tokio::test]
async fn the_chair_the_deputy_the_lead_and_the_bot_are_directory_people() {
    let s = sample();
    for (key, n) in [
        ("Alice Chair", 1_u8),
        ("Bob Deputy", 10),
        ("Carol Lead", 12),
        ("Assistant Bot", 4),
    ] {
        let person = s.person(key).expect("person");
        let in_directory = sample_ref(n).expect("entry");
        assert_eq!(
            person.id, in_directory.id,
            "{key} keeps its directory reference"
        );
        let shown = s
            .people
            .display(&person)
            .await
            .expect("display")
            .expect("known");
        assert_eq!(
            Some(shown.0.as_str()),
            sample_name(n),
            "{key} shows the directory name"
        );
    }
}

#[test]
fn the_other_people_stay_invented_here() {
    let s = sample();
    let gina = s.person("Gina Guest").expect("person");
    assert!((1..=12).filter_map(sample_ref).all(|p| p.id != gina.id));
}
