//! `sample()`: the shared test data of the consumers (calendar adapter, tasks, groups): twelve entries that cover
//! every state of `person_directory_v1`.
//!
//! Pattern: Object Mother (one well-known directory, entries addressed by number through `sample_ref`).
//!
//! | n | art | what |
//! | --- | --- | --- |
//! | 1 | Person | Amara Zolenko, confirmed, `mailto:amara@example.org`, the asker, JSContact-style excerpt: 2 e-mails, 2 phones, postal address, photo placeholder (`avatar`) |
//! | 2 | Profile | "Amara Z.", linked to 1 (`describes`), same address as 1 |
//! | 3 | Profile | "A. Zolenko", unlinked, same address as 1 (two profiles with one address) |
//! | 4 | Person | "Assistent Sol", `ai` set |
//! | 5 | Group | "Quillon Team" (members live in fs-groups) |
//! | 6 | Organization | "Tamarind Works Ltd" |
//! | 7 | Person | "Brannoc Deleted", deleted (tombstone) |
//! | 8 | Person | "Ilvy Hidden", no longer shared |
//! | 9 | Profile | "Soo-jin M.", merged into 10 |
//! | 10 | Person | "Soo-jin Marchetti", `tel:+49305550123` |
//! | 11 | Profile | "Import Tariq", `Proposed`, `mailto:tariq@example.org` |
//! | 12 | Person | "Tariq Lindqvist", confirmed, e-mail and phone in the excerpt |

use super::{Entry, FakeDirectory};
use crate::person_directory_v1::{EntryState, ExcerptValue, PersonArt, PersonRef};
use fs_types::{
    Address, AddressKind, BasicValue, ContactAddress, EmailAddress, FileRef, MediaType, Name,
    PhoneNumber, Reference, Text,
};

/// The display names of the twelve entries, by number (1 to 12): the ONE place the sample names live. Other samples
/// (`fs-events`, `fs-group-directory`) point at these people by their reference and take the name from here.
pub const SAMPLE_NAMES: [&str; 12] = [
    "Amara Zolenko",
    "Amara Z.",
    "A. Zolenko",
    "Assistent Sol",
    "Quillon Team",
    "Tamarind Works Ltd",
    "Brannoc Deleted",
    "Ilvy Hidden",
    "Soo-jin M.",
    "Soo-jin Marchetti",
    "Import Tariq",
    "Tariq Lindqvist",
];

/// The display name of entry `n` (1 to 12) of [`sample`]; `None` for any other number.
#[must_use]
pub fn sample_name(n: u8) -> Option<&'static str> {
    SAMPLE_NAMES.get(usize::from(n).checked_sub(1)?).copied()
}

/// The reference of entry `n` (1 to 12) of [`sample`], with the art of that entry; `None` for any other number.
#[must_use]
pub fn sample_ref(n: u8) -> Option<PersonRef> {
    let art = match n {
        2 | 3 | 9 | 11 => PersonArt::Profile,
        5 => PersonArt::Group,
        6 => PersonArt::Organization,
        1 | 4 | 7 | 8 | 10 | 12 => PersonArt::Person,
        _ => return None,
    };
    let id = Reference::parse(&format!("urn:uuid:00000000-0000-7000-8000-{n:012x}")).ok()?;
    Some(PersonRef {
        id,
        art,
        abzug: None,
    })
}

fn row(n: u8, address: Option<&str>) -> Option<Entry> {
    let mut e = Entry::new(sample_ref(n)?, sample_name(n)?);
    e.address = address.and_then(|a| ContactAddress::parse(a).ok());
    Some(e)
}

fn mail(text: &str) -> Option<ExcerptValue> {
    EmailAddress::parse(text).ok().map(ExcerptValue::Email)
}

fn phone(text: &str) -> Option<ExcerptValue> {
    PhoneNumber::parse(text).ok().map(ExcerptValue::Phone)
}

/// The photo placeholder of entry 1 (a file reference with an all-zero checksum: no real bytes exist).
fn photo_placeholder() -> Option<FileRef> {
    Some(FileRef {
        iri: Reference::parse("urn:fs-testdata:photo:amara-zolenko").ok()?,
        sha256: [0; 32],
        media_type: MediaType("image/jxl".to_owned()),
    })
}

/// The contact fields of entry `n` as JSContact (RFC 9553) has them, ALL of them (the trait `excerpt` hands out one
/// value per field; this is the whole card a consumer picture needs): entry 1 has name, surname, TWO e-mails, TWO phones
/// and a postal address; entry 12 has name, one e-mail and one phone; every other entry has none. The birthday
/// (`anniversaries`, RFC 9553 2.8.1) is not an `ExcerptField` of `person_directory_v1` yet; it needs an appended
/// field first (see the CHANGELOG), so it is not invented here.
#[must_use]
pub fn sample_card(n: u8) -> Vec<ExcerptValue> {
    let mut out = Vec::new();
    match n {
        1 => {
            out.extend(
                Name::new("Amara Zolenko", &[])
                    .ok()
                    .map(ExcerptValue::FullName),
            );
            out.extend(Text::parse("Zolenko").ok().map(ExcerptValue::Surname));
            out.extend(mail("amara@example.org"));
            out.extend(mail("amara.zolenko@arbeit.example.org"));
            out.extend(phone("+49 30 5550100"));
            out.extend(phone("+49 171 5550101"));
            let home = [
                (AddressKind::Name, "Musterweg"),
                (AddressKind::Number, "12"),
                (AddressKind::Postcode, "10115"),
                (AddressKind::Locality, "Berlin"),
            ];
            out.extend(
                Address::new(&home, "DE", "Musterweg 12\n10115 Berlin")
                    .ok()
                    .map(ExcerptValue::Postal),
            );
        }
        12 => {
            out.extend(
                Name::new("Tariq Lindqvist", &[])
                    .ok()
                    .map(ExcerptValue::FullName),
            );
            out.extend(mail("tariq.lindqvist@example.org"));
            out.extend(phone("+46 8 5550123"));
        }
        _ => {}
    }
    out
}

/// Entry 1 with its card and the photo placeholder (`avatar`).
fn amara() -> Option<Entry> {
    let mut e = row(1, Some("mailto:amara@example.org"))?;
    e.excerpt = sample_card(1);
    e.avatar = photo_placeholder();
    Some(e)
}

/// Entry 12 with its card: a second person with contact fields.
fn tariq() -> Option<Entry> {
    let mut e = row(12, None)?;
    e.excerpt = sample_card(12);
    Some(e)
}

/// A directory with the twelve entries of the table above; the asker is entry 1. Entries 7, 8 and 9 are put into
/// their state after creation, so the change log holds `Deleted`, `NotShared` and `Merged`.
#[must_use]
pub fn sample() -> FakeDirectory {
    let d = FakeDirectory::new().with_asker(sample_ref(1));
    let linked = row(2, Some("mailto:amara@example.org")).map(|mut e| {
        e.describes = sample_ref(1);
        e
    });
    let ai = row(4, None).map(|mut e| {
        e.ai = true;
        e
    });
    let proposed = row(11, Some("mailto:tariq@example.org")).map(|mut e| {
        e.state = EntryState::Proposed;
        e
    });
    let rows = [
        amara(),
        linked,
        row(3, Some("mailto:amara@example.org")),
        ai,
        row(5, None),
        row(6, None),
        row(7, None),
        row(8, None),
        row(9, None),
        row(10, Some("tel:+49305550123")),
        proposed,
        tariq(),
    ];
    for e in rows.into_iter().flatten() {
        d.add(e);
    }
    if let (Some(deleted), Some(hidden), Some(from), Some(into)) =
        (sample_ref(7), sample_ref(8), sample_ref(9), sample_ref(10))
    {
        d.delete(&deleted);
        d.unshare(&hidden);
        d.merge(&from, &into);
    }
    d
}
