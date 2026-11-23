//! Addons of a group (composition): `addons`, `attach_addon`, `update_addon`, `detach_addon`.
//!
//! Pattern: Command handlers on the state. An addon is only attached: no rule of `access.rs`, `nesting_logic` or the
//! member views asks for one (the conformance rules compare members, rights and edges before and after).

use crate::groups_v1::shared::{ObjectId, Revision};
use crate::groups_v1::{Error, Field, FieldProblem};
use crate::groups_v2::{Addon, AddonKind, AddonVerb, Error2, Level, Result2};
use crate::state::State;
use crate::validate::{NAME_MAX, text};

impl State {
    /// Good input: the legal name is a good text; optional fields too.
    fn check_addon(kind: &AddonKind) -> Result2<()> {
        let AddonKind::FormalOrganization(f) = kind else {
            return Ok(());
        };
        let mut problems: Vec<FieldProblem> = Vec::new();
        text(Field::Name, &f.legal_name, NAME_MAX, &mut problems);
        for v in [&f.legal_form, &f.registration].into_iter().flatten() {
            text(Field::Description, v, NAME_MAX, &mut problems);
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid(problems).into())
        }
    }

    /// The addon, when it hangs at a group the asker may read (else `NoSuchAddon`).
    fn addon_of(&self, id: ObjectId) -> Result2<Addon> {
        let a = self
            .v2
            .addons
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or(Error2::NoSuchAddon(id))?;
        self.v2_gate(a.group).map_err(|_| Error2::NoSuchAddon(id))?;
        Ok(a)
    }

    pub(crate) fn addons(&self, g: ObjectId) -> Result2<Vec<Addon>> {
        self.v2_gate(g)?;
        Ok(self
            .v2
            .addons
            .iter()
            .filter(|a| a.group == g)
            .cloned()
            .collect())
    }

    pub(crate) fn attach_addon(&mut self, g: ObjectId, kind: AddonKind) -> Result2<Addon> {
        self.v2_gate(g)?;
        self.need_level(g, Level::Manager)?;
        Self::check_addon(&kind)?;
        let iri = kind.iri();
        if let Some(a) = self
            .v2
            .addons
            .iter()
            .find(|a| a.group == g && a.kind.iri() == iri)
        {
            return Err(Error2::AddonAlreadyThere(a.id));
        }
        let now = self.tick();
        let addon = Addon {
            id: self.fresh(),
            revision: Revision(1),
            group: g,
            kind,
            attached_by: self.asker.clone(),
            attached: now,
        };
        self.v2.addons.push(addon.clone());
        self.note_addon(g, AddonVerb::Attached, addon.kind.iri());
        Ok(addon)
    }

    pub(crate) fn update_addon(
        &mut self,
        id: ObjectId,
        expected: Revision,
        kind: AddonKind,
    ) -> Result2<Addon> {
        let old = self.addon_of(id)?;
        self.need_level(old.group, Level::Manager)?;
        Self::cas(old.revision, expected)?;
        Self::check_addon(&kind)?;
        if kind.iri() != old.kind.iri() {
            return Err(Error::Invalid(vec![FieldProblem {
                field: Field::Kind,
                problem: crate::groups_v1::ProblemKind::BadCharacters,
            }])
            .into());
        }
        self.tick();
        let row = self
            .v2
            .addons
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(Error2::NoSuchAddon(id))?;
        row.kind = kind;
        row.revision = Revision(old.revision.0 + 1);
        let done = row.clone();
        self.note_addon(done.group, AddonVerb::Updated, done.kind.iri());
        Ok(done)
    }

    pub(crate) fn detach_addon(&mut self, id: ObjectId, expected: Revision) -> Result2<()> {
        let old = self.addon_of(id)?;
        self.need_level(old.group, Level::Manager)?;
        Self::cas(old.revision, expected)?;
        self.tick();
        self.v2.addons.retain(|a| a.id != id);
        self.note_addon(old.group, AddonVerb::Detached, old.kind.iri());
        Ok(())
    }
}
