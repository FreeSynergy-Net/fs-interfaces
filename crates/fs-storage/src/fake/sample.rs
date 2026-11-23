//! The sample tree: "My files" of Anna, ONE group folder (an example local group of Helfa), 15 files of mixed types, a trash.
//!
//! Pattern: Builder (fixed sample data, deterministic ids and times).
//!
//! Anna owns the group, Ben is a member, Fremd is nobody, Admin is a manager (admin level (a)/(b) without server access: structure of groups, only `usage` of
//! private folders), Operator has server access and reads everything, Bot is an AI actor (member on paper, no right, no folder). Every person has "My files" (Ben's holds one file,
//! for the manager and operator tests). Bot also stands in the operator list and still gets nothing.
//!
//! The group follows `helfa/UMFRAGE-ergebnis.md` (23.11.2026) and nothing else of Helfa: a local group ("Ortsgruppe") with
//! the 4+1 rules, the three conflict steps and a moderator team. Everything the survey does not say (member lists, minutes,
//! photos, Anna's private files) has neutral example names. The group folder's name ends in "(Beispieldaten)", and so does
//! every example file that pretends to be real data. Three images are made by `capture` (`generator = "capture"`) and lie in
//! three different places: that is the gallery, a `scope`, no folder. Trash: Alt.txt.

use super::state::{Group, Node, World};
use crate::entry::{Entry, Kind, Root, RootKind};
use crate::ids::EntryId;
use fs_types::{BasicValue, FileRef, MediaType, PersonArt, PersonRef, Reference, Utc};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const GROUP_NAME: &str = "Helfa Ortsgruppe (Beispieldaten)";

/// A `FileRef` for `bytes`: the reference is the checksum (`urn:sha256:<hex>`), as in the v5 `FileStore`.
pub(super) fn make_ref(bytes: &[u8], media: &str) -> FileRef {
    let sha: [u8; 32] = Sha256::digest(bytes).into();
    let hex = sha.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    FileRef {
        iri: Reference::parse(&format!("urn:sha256:{hex}")).expect("fixed iri"),
        sha256: sha,
        media_type: MediaType(media.to_owned()),
    }
}

/// The people of the sample.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Owner of "My files" and of the group.
    pub anna: PersonRef,
    /// Member of the group.
    pub ben: PersonRef,
    /// Nobody: no right anywhere.
    pub fremd: PersonRef,
    /// Manager (admin level (a)/(b), no server access): structure of group folders, only `usage` of private ones.
    pub admin: PersonRef,
    /// Operator (server access): reads everything, private folders too.
    pub operator: PersonRef,
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
        operator: pref(6, PersonArt::Person),
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
            generator: None,
        };
        let node = Node {
            entry,
            parent: parent.map(str::to_owned),
            trashed_from: None,
        };
        self.nodes.insert(id.to_owned(), node);
    }

    pub(super) fn add_file(
        &mut self,
        parent: &str,
        name: &str,
        media: &str,
        bytes: &[u8],
        owner: &PersonRef,
        generator: Option<&str>,
    ) -> String {
        let id = self.next_id("n");
        let file_ref = make_ref(bytes, media);
        self.blobs.insert(file_ref.iri.canonical(), bytes.to_vec());
        let entry = Entry {
            id: EntryId(id.clone()),
            name: name.to_owned(),
            kind: Kind::File,
            size: Some(bytes.len() as u64),
            modified: Some(self.now()),
            media_type: Some(MediaType(media.to_owned())),
            file_ref: Some(file_ref),
            owner: owner.clone(),
            etag: Some(self.seq.to_string()),
            link_target: None,
            generator: generator.map(str::to_owned),
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

#[allow(clippy::too_many_lines)] // fixed sample data
pub(super) fn build() -> World {
    let s = sample();
    let mut w = World::default();
    for (who, root) in [
        (&s.anna, "mine-anna"),
        (&s.ben, "mine-ben"),
        (&s.fremd, "mine-fremd"),
        (&s.admin, "mine-admin"),
        (&s.operator, "mine-operator"),
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
    w.add_folder("grp-helfa", None, GROUP_NAME, &s.helfa);
    w.roots.push(Root {
        id: EntryId("grp-helfa".into()),
        name: GROUP_NAME.into(),
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
    w.operators.push(s.operator.clone());
    w.operators.push(s.bot.clone());
    w.bots.push(s.bot.clone());
    for (id, parent, name, owner) in [
        ("f-dok", "mine-anna", "Dokumente", &s.anna),
        ("f-bild", "mine-anna", "Bilder", &s.anna),
        ("f-musik", "mine-anna", "Musik", &s.anna),
        ("f-regeln", "grp-helfa", "Regeln", &s.helfa),
        ("f-mod", "grp-helfa", "Moderation", &s.helfa),
        ("f-foto", "grp-helfa", "Fotos", &s.helfa),
    ] {
        w.add_folder(id, Some(parent), name, owner);
    }
    let odt = "application/vnd.oasis.opendocument.text";
    let files: [(&str, &str, &str, &PersonRef, Option<&str>); 15] = [
        (
            "mine-anna",
            "Aufnahme-1.png",
            "image/png",
            &s.anna,
            Some("capture"),
        ),
        (
            "f-bild",
            "Aufnahme-2.png",
            "image/png",
            &s.anna,
            Some("capture"),
        ),
        (
            "f-foto",
            "Aufnahme-3.png",
            "image/png",
            &s.helfa,
            Some("capture"),
        ),
        ("mine-ben", "Privat.txt", "text/plain", &s.ben, None),
        ("f-dok", "Lebenslauf.pdf", "application/pdf", &s.anna, None),
        ("f-dok", "Notizen.md", "text/markdown", &s.anna, None),
        (
            "f-dok",
            "Budget.ods",
            "application/vnd.oasis.opendocument.spreadsheet",
            &s.anna,
            None,
        ),
        ("f-bild", "Urlaub.jpg", "image/jpeg", &s.anna, None),
        ("f-bild", "Logo.svg", "image/svg+xml", &s.anna, None),
        ("f-musik", "Lied.ogg", "audio/ogg", &s.anna, None),
        (
            "f-regeln",
            "Vier-plus-eins-Regeln.md",
            "text/markdown",
            &s.helfa,
            None,
        ),
        (
            "f-regeln",
            "Konfliktstufen.md",
            "text/markdown",
            &s.helfa,
            None,
        ),
        (
            "f-mod",
            "Moderatorenteam (Beispieldaten).csv",
            "text/csv",
            &s.helfa,
            None,
        ),
        ("f-mod", "Sitzung (Beispieldaten).odt", odt, &s.helfa, None),
        (
            "f-foto",
            "Fest (Beispieldaten).mp4",
            "video/mp4",
            &s.helfa,
            None,
        ),
    ];
    for (parent, name, media, owner, generator) in files {
        w.add_file(parent, name, media, &body(name), owner, generator);
    }
    let alt = w.add_file(
        "f-dok",
        "Alt.txt",
        "text/plain",
        &body("Alt.txt"),
        &s.anna,
        None,
    );
    if let Some(n) = w.nodes.get_mut(&alt) {
        n.trashed_from = Some("f-dok".into());
    }
    let mut link = w.nodes["f-dok"].entry.clone();
    link.id = EntryId("l-helfa".into());
    link.name = "Zum Helfa-Ordner".into();
    link.kind = Kind::Link;
    link.link_target = Some(EntryId("grp-helfa".into()));
    let node = Node {
        entry: link,
        parent: Some("mine-anna".into()),
        trashed_from: None,
    };
    w.nodes.insert("l-helfa".into(), node);
    w
}
