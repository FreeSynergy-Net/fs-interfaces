//! The sample tree: "My files" of Anna, ONE group folder (Helfa) with the usual shape, 12 files of mixed types, a trash.
//!
//! Pattern: Builder (fixed sample data, deterministic ids and times; the bytes go through `fs-objects` `MemoryFiles`).
//!
//! Anna owns the group, Ben is a member, Fremd is nobody, Admin administers (structure of groups, only `usage` of private
//! folders), Bot is an AI actor (member of the group on paper, no right, no folder). Every person has "My files" (Ben's holds
//! one file, for the admin test). Anna's "Aufnahmen" is a gallery of 3 `Image` files owned by the file module's rule (F-N2
//! recommendation: files in "My files", not a program tree). Layout (the Helfa shape is an assumption: protocols, photos,
//! the statutes; no reference structure was found in the docs): Mine = Dokumente (3 files), Bilder (2), Musik (1), a
//! link to the group folder; Helfa = Protokolle (2), Fotos (2), Satzung.pdf; Trash = Alt.txt. 15 files in Anna's tree (12 plus the 3 images).

use super::state::{Group, Node, World};
use crate::entry::{Entry, Kind, Root, RootKind};
use crate::ids::EntryId;
use fs_types::{BasicValue, MediaType, PersonArt, PersonRef, Reference, Utc};

/// The people of the sample.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Owner of "My files" and of the group.
    pub anna: PersonRef,
    /// Member of the group.
    pub ben: PersonRef,
    /// Nobody: no right anywhere.
    pub fremd: PersonRef,
    /// Administrator: structure of group folders, only `usage` of private ones.
    pub admin: PersonRef,
    /// An AI actor: no folder, no right, never an owner.
    pub bot: PersonRef,
    /// The group "Helfa".
    pub helfa: PersonRef,
}

fn pref(n: u8, art: PersonArt) -> PersonRef {
    let id = Reference::parse(&format!(
        "urn:uuid:00000000-0000-7000-8000-0000000000{n:02}"
    ))
    .expect("fixed iri");
    PersonRef {
        id,
        art,
        abzug: None,
    }
}

/// The people of the sample (the same in every call).
#[must_use]
pub fn sample() -> Sample {
    Sample {
        anna: pref(1, PersonArt::Person),
        ben: pref(2, PersonArt::Person),
        fremd: pref(3, PersonArt::Person),
        admin: pref(4, PersonArt::Person),
        bot: pref(5, PersonArt::Person),
        helfa: pref(10, PersonArt::Group),
    }
}

impl World {
    pub(super) fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}{:04}", self.seq)
    }

    pub(super) fn now(&self) -> Utc {
        Utc::from_millis(1_790_000_000_000 + i64::from(self.seq) * 1000)
    }

    pub(super) fn add_folder(
        &mut self,
        id: &str,
        parent: Option<&str>,
        name: &str,
        owner: &PersonRef,
    ) {
        let entry = Entry {
            id: EntryId(id.to_owned()),
            name: name.to_owned(),
            kind: Kind::Folder,
            size: None,
            modified: Some(self.now()),
            media_type: None,
            file_ref: None,
            owner: owner.clone(),
            etag: Some(self.seq.to_string()),
            link_target: None,
        };
        self.nodes.insert(
            id.to_owned(),
            Node {
                entry,
                parent: parent.map(str::to_owned),
                trashed_from: None,
            },
        );
    }

    pub(super) fn add_file(
        &mut self,
        parent: &str,
        name: &str,
        media: &str,
        bytes: Vec<u8>,
        owner: &PersonRef,
    ) -> String {
        let id = self.next_id("n");
        let entry = Entry {
            id: EntryId(id.clone()),
            name: name.to_owned(),
            kind: Kind::File,
            size: Some(bytes.len() as u64),
            modified: Some(self.now()),
            media_type: Some(MediaType(media.to_owned())),
            file_ref: self.files.put(&bytes, &MediaType(media.to_owned())).ok(),
            owner: owner.clone(),
            etag: Some(self.seq.to_string()),
            link_target: None,
        };
        let node = Node {
            entry,
            parent: Some(parent.to_owned()),
            trashed_from: None,
        };
        self.nodes.insert(id.clone(), node);
        id
    }
}

fn body(name: &str) -> Vec<u8> {
    format!("sample content of {name}\n").repeat(3).into_bytes()
}

pub(super) fn build() -> World {
    let s = sample();
    let mut w = World::default();
    for (who, root) in [
        (&s.anna, "mine-anna"),
        (&s.ben, "mine-ben"),
        (&s.fremd, "mine-fremd"),
        (&s.admin, "mine-admin"),
    ] {
        w.add_folder(root, None, "Meine Dateien", who);
        w.roots.push(Root {
            id: EntryId(root.to_owned()),
            name: "Meine Dateien".into(),
            kind: RootKind::Mine,
            owner: who.clone(),
            usage: None,
        });
    }
    w.add_folder("grp-helfa", None, "Helfa", &s.helfa);
    w.roots.push(Root {
        id: EntryId("grp-helfa".into()),
        name: "Helfa".into(),
        kind: RootKind::Group,
        owner: s.helfa.clone(),
        usage: None,
    });
    w.groups.push(Group {
        who: s.helfa.clone(),
        owners: vec![s.anna.clone()],
        members: vec![s.anna.clone(), s.ben.clone(), s.bot.clone()],
    });
    w.admins.push(s.admin.clone());
    w.bots.push(s.bot.clone());
    for (id, parent, name, owner) in [
        ("f-dok", "mine-anna", "Dokumente", &s.anna),
        ("f-bild", "mine-anna", "Bilder", &s.anna),
        ("f-musik", "mine-anna", "Musik", &s.anna),
        ("f-aufn", "mine-anna", "Aufnahmen", &s.anna),
        ("f-prot", "grp-helfa", "Protokolle", &s.helfa),
        ("f-foto", "grp-helfa", "Fotos", &s.helfa),
    ] {
        w.add_folder(id, Some(parent), name, owner);
    }
    let files: [(&str, &str, &str, &PersonRef); 15] = [
        ("f-aufn", "Aufnahme-1.png", "image/png", &s.anna),
        ("f-aufn", "Aufnahme-2.png", "image/png", &s.anna),
        ("f-aufn", "Aufnahme-3.png", "image/png", &s.anna),
        ("mine-ben", "Privat.txt", "text/plain", &s.ben),
        ("f-dok", "Lebenslauf.pdf", "application/pdf", &s.anna),
        ("f-dok", "Notizen.md", "text/markdown", &s.anna),
        (
            "f-dok",
            "Budget.ods",
            "application/vnd.oasis.opendocument.spreadsheet",
            &s.anna,
        ),
        ("f-bild", "Urlaub.jpg", "image/jpeg", &s.anna),
        ("f-bild", "Logo.svg", "image/svg+xml", &s.anna),
        ("f-musik", "Lied.ogg", "audio/ogg", &s.anna),
        (
            "f-prot",
            "Sitzung-2026-11.odt",
            "application/vnd.oasis.opendocument.text",
            &s.helfa,
        ),
        ("f-prot", "Mitglieder.csv", "text/csv", &s.helfa),
        ("f-foto", "Fest.png", "image/png", &s.helfa),
        ("f-foto", "Fest.mp4", "video/mp4", &s.helfa),
        ("grp-helfa", "Satzung.pdf", "application/pdf", &s.helfa),
    ];
    for (parent, name, media, owner) in files {
        w.add_file(parent, name, media, body(name), owner);
    }
    let alt = w.add_file("f-dok", "Alt.txt", "text/plain", body("Alt.txt"), &s.anna);
    if let Some(n) = w.nodes.get_mut(&alt) {
        n.trashed_from = Some("f-dok".into());
    }
    let mut link = w.nodes["f-dok"].entry.clone();
    link.id = EntryId("l-helfa".into());
    link.name = "Zum Helfa-Ordner".into();
    link.kind = Kind::Link;
    link.link_target = Some(EntryId("grp-helfa".into()));
    w.nodes.insert(
        "l-helfa".into(),
        Node {
            entry: link,
            parent: Some("mine-anna".into()),
            trashed_from: None,
        },
    );
    w
}
