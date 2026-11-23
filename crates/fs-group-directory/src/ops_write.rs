//! Writing groups: `create`, `update`, and the AI and sensitivity checks they share.
//!
//! Pattern: Command handlers on the state (check rights, then the revision, then the input, then write and log).

use crate::groups_v1::shared::{ObjectId, Revision};
use crate::groups_v1::{
    ChangeKind, ConflictKind, Error, Field, FieldProblem, Grant, Group, GroupDraft, GroupField,
    GroupPatch, HistoryKind, InForce, Result, Right,
};
use crate::ids::reference_of;
use crate::state::{Row, State};
use crate::validate::{NAME_MAX, TEXT_MAX, mirror, text};

impl State {
    pub(crate) fn create(&mut self, d: GroupDraft) -> Result<Group> {
        match self.people.status(&self.asker) {
            Some(true) => return Err(Error::AgentNotAdmin),
            None => return Err(Error::AiStatusUnknown),
            Some(false) => {}
        }
        let mut problems = Vec::new();
        text(Field::Name, &d.name, NAME_MAX, &mut problems);
        optional(Field::Kind, d.kind.as_deref(), NAME_MAX, &mut problems);
        optional(
            Field::Description,
            d.description.as_deref(),
            TEXT_MAX,
            &mut problems,
        );
        mirror(d.mirror, d.sensitive, d.ai, &mut problems);
        if !problems.is_empty() {
            return Err(Error::Invalid(problems));
        }
        let now = self.tick();
        let id = self.fresh();
        let group = Group {
            id,
            reference: reference_of(id),
            revision: Revision(1),
            art: d.art,
            name: d.name,
            kind: d.kind,
            description: d.description,
            default_mode: d.default_mode,
            visibility: d.visibility,
            sensitive: d.sensitive,
            ai: d.ai,
            mirror: d.mirror,
            created: now,
            created_by: self.asker.clone(),
            updated: now,
        };
        self.groups.insert(
            id,
            Row {
                group: group.clone(),
                deleted: None,
                rev_at_delete: Revision(1),
            },
        );
        let grant_id = self.fresh();
        self.grants.push(Grant {
            id: grant_id,
            revision: Revision(1),
            group: id,
            holder: self.asker.clone(),
            right: Right::Manage,
            granted_by: self.asker.clone(),
            granted: now,
            in_force: InForce::Yes,
        });
        self.log(id, HistoryKind::Created);
        self.change(ChangeKind::Group(id));
        Ok(group)
    }

    pub(crate) fn update(
        &mut self,
        id: ObjectId,
        expected: Revision,
        p: GroupPatch,
    ) -> Result<Group> {
        let old = self.visible(id)?.clone();
        self.need(id, Right::Manage)?;
        Self::cas(old.revision, expected)?;
        let mut new = old.clone();
        let mut fields = Vec::new();
        let mut problems: Vec<FieldProblem> = Vec::new();
        if let Some(v) = p.art.filter(|v| *v != old.art) {
            new.art = v;
            fields.push(GroupField::Art);
        }
        if let Some(v) = p.name.filter(|v| *v != old.name) {
            text(Field::Name, &v, NAME_MAX, &mut problems);
            new.name = v;
            fields.push(GroupField::Name);
        }
        if let Some(v) = p.kind.filter(|v| *v != old.kind) {
            optional(Field::Kind, v.as_deref(), NAME_MAX, &mut problems);
            new.kind = v;
            fields.push(GroupField::Kind);
        }
        if let Some(v) = p.description.filter(|v| *v != old.description) {
            optional(Field::Description, v.as_deref(), TEXT_MAX, &mut problems);
            new.description = v;
            fields.push(GroupField::Description);
        }
        if let Some(v) = p.default_mode.filter(|v| *v != old.default_mode) {
            new.default_mode = v;
            fields.push(GroupField::DefaultMode);
        }
        if let Some(v) = p.visibility.filter(|v| *v != old.visibility) {
            new.visibility = v;
            fields.push(GroupField::Visibility);
        }
        if let Some(v) = p.sensitive.filter(|v| *v != old.sensitive) {
            new.sensitive = v;
            fields.push(GroupField::Sensitive);
        }
        if let Some(v) = p.ai.filter(|v| *v != old.ai) {
            new.ai = v;
            fields.push(GroupField::Ai);
        }
        if let Some(v) = p.mirror.filter(|v| *v != old.mirror) {
            new.mirror = v;
            fields.push(GroupField::Mirror);
        }
        mirror(new.mirror, new.sensitive, new.ai, &mut problems);
        if !problems.is_empty() {
            return Err(Error::Invalid(problems));
        }
        if fields.is_empty() {
            return Ok(old);
        }
        if new.ai != old.ai {
            self.check_ai_switch(id, new.ai)?;
        }
        if new.sensitive != old.sensitive {
            self.check_sensitive_switch(id, new.sensitive)?;
        }
        new.updated = self.tick();
        new.revision = Revision(old.revision.0 + 1);
        if let Some(row) = self.groups.get_mut(&id) {
            row.group = new.clone();
        }
        self.log(id, HistoryKind::Updated { fields });
        self.change(ChangeKind::Group(id));
        Ok(new)
    }

    /// `update(ai)`: `true` needs no human member and no effective edge or child; `false` no AI member.
    fn check_ai_switch(&self, id: ObjectId, to_ai: bool) -> Result<()> {
        let refused = |path: Vec<ObjectId>| Error::AiNesting { group: id, path };
        let status = |m: &&crate::groups_v1::Membership| self.people.status(&m.person);
        let mine = || self.members.iter().filter(|m| m.group == id);
        if to_ai {
            if mine().any(|m| status(&m) != Some(true)) {
                return Err(refused(vec![id]));
            }
            let bad = self.edges.iter().find(|e| {
                e.parent == id || (e.child == id && e.mode != crate::groups_v1::NestingMode::Single)
            });
            if let Some(e) = bad {
                return Err(refused(vec![e.child, e.parent]));
            }
        } else if mine().any(|m| status(&m) == Some(true)) {
            return Err(refused(vec![id]));
        }
        Ok(())
    }

    /// `update(sensitive)`: no arc between a sensitive and a non-sensitive group, either way.
    fn check_sensitive_switch(&self, id: ObjectId, to: bool) -> Result<()> {
        let sensitive = |g: ObjectId| {
            if g == id {
                to
            } else {
                self.groups.get(&g).is_some_and(|r| r.group.sensitive)
            }
        };
        for e in &self.edges {
            if let Some((u, v)) = self.logic_edge(e).arc()
                && sensitive(u) != sensitive(v)
            {
                return Err(Error::ModeConflict(ConflictKind::SensitiveFlow {
                    from: u,
                    to: v,
                }));
            }
        }
        Ok(())
    }
}

/// An optional text: `Some` must be a good text.
pub(crate) fn optional(field: Field, value: Option<&str>, max: usize, out: &mut Vec<FieldProblem>) {
    if let Some(v) = value {
        text(field, v, max, out);
    }
}
