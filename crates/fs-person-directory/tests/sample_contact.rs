//! The contact fields and the names of `sample()` (JSContact, RFC 9553).
//!
//! Pattern: none (tests). Red when a field of the card or a name goes missing.

use fs_person_directory::{
    ExcerptField, ExcerptValue, PersonDirectory, Resolved, SAMPLE_NAMES, sample, sample_address,
    sample_card, sample_name, sample_ref,
};
use fs_types::{ContactAddress, ContactValue};

fn count(card: &[ExcerptValue], field: ExcerptField) -> usize {
    card.iter().filter(|v| v.field() == field).count()
}

#[test]
fn entry_one_has_several_phones_and_mails_and_a_postal_address() {
    let card = sample_card(1);
    assert_eq!(count(&card, ExcerptField::Email), 2);
    assert_eq!(count(&card, ExcerptField::Phone), 2);
    assert_eq!(count(&card, ExcerptField::Postal), 1);
    assert_eq!(count(&card, ExcerptField::FullName), 1);
    assert!(
        sample_card(2).is_empty(),
        "a profile has no card of its own"
    );
}

#[test]
fn a_second_person_has_contact_fields_too() {
    let card = sample_card(12);
    assert_eq!(count(&card, ExcerptField::Email), 1);
    assert_eq!(count(&card, ExcerptField::Phone), 1);
}

#[test]
fn the_names_have_one_table_and_the_display_is_taken_from_it() {
    assert_eq!(SAMPLE_NAMES.len(), 12);
    assert_eq!(sample_name(0), None);
    assert_eq!(sample_name(13), None);
    assert_eq!(sample_name(1), Some("Amara Zolenko"));
}

#[tokio::test]
async fn the_directory_shows_the_table_names_and_the_photo_placeholder() {
    let d = sample();
    for n in 1..=12_u8 {
        let r = sample_ref(n).expect("entry");
        let shown = match d.resolve(&r).await.expect("resolve") {
            Resolved::Found(v) => Some(v),
            _ => None,
        };
        // Entries 7 (deleted), 8 (not shared) and 9 (merged) do not show their own name.
        if let Some(v) = shown.filter(|_| ![7, 8, 9].contains(&n)) {
            assert_eq!(Some(v.display.as_str()), sample_name(n), "entry {n}");
            assert_eq!(
                v.avatar.is_some(),
                n == 1,
                "only entry 1 has a photo placeholder"
            );
        }
    }
    let x = d
        .excerpt(
            &sample_ref(1).expect("entry"),
            &[ExcerptField::Email, ExcerptField::Postal],
        )
        .await
        .expect("excerpt");
    assert_eq!(x.values.len(), 2);
}

#[tokio::test]
async fn the_address_of_the_shared_people_is_in_the_directory() {
    let d = sample();
    for n in [1_u8, 4, 10, 12] {
        let wanted = sample_address(n).expect("an address");
        let found = d
            .find_by_address(&ContactAddress::parse(wanted).expect("address"))
            .await
            .expect("find");
        let me = sample_ref(n).expect("entry");
        assert!(
            found.iter().any(|p| p.person.id == me.id),
            "entry {n}: {wanted}"
        );
        // The card carries the same address as its first e-mail (when it has a card).
        let card = sample_card(n);
        if let Some(ExcerptValue::Email(first)) =
            card.iter().find(|v| v.field() == ExcerptField::Email)
        {
            assert_eq!(
                format!("mailto:{}", first.as_str()),
                wanted,
                "card of entry {n}"
            );
        }
    }
    assert_eq!(sample_address(2), None);
}
