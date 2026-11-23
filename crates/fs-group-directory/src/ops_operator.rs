//! Operators (scope `Server`, user decision 16): the list everybody sees and the LOCAL way in (`LocalOperator`).
//!
//! Pattern: Command handlers on the state. There is NO contract path here: the only writers are `operator_set_local`
//! and `operator_unset_local` (called by `fake_operator.rs`); every contract path is closed by `decide_base`
//! (no write right at the operator group, not even through an intervention), `guard_role_grant` (the role
//! `operator`), `guard_operator_membership` and `grant_scope` (the scope `Server`).

use crate::groups::shared::{ObjectId, PersonRef, Revision};
use crate::groups::{
    ChangeKind, Error, Group, HistoryKind, Missing, NestingMode, OPERATOR_GROUP_NAME, OperatorList,
    Result, Scope, ScopeGrant, Visibility, check_operator_candidate, role_operator,
};
use crate::ids::reference_of;
use crate::state::{Row, State};

impl State {
    /// Whether `g` is the group "Betreiber".
    pub(crate) fn is_operator_group(&self, g: ObjectId) -> bool {
        self.gov.operator_group == Some(g)
    }

    /// The operators now: a membership with the role `operator`, and the mark still says "human, not supervised" (a
    /// mark that changes later or is not known costs the operator role at once: fail-closed).
    fn operator_persons(&self) -> Vec<PersonRef> {
        let Some(g) = self.gov.operator_group else {
            return Vec::new();
        };
        let role = role_operator();
        let mut rows: Vec<_> = self
            .members
            .iter()
            .filter(|m| {
                m.group == g
                    && m.role.as_ref() == Some(&role)
                    && check_operator_candidate(
                        m.person.art,
                        self.people.status(&m.person),
                        self.people.supervised(&m.person),
                    )
                    .is_ok()
            })
            .collect();
        rows.sort_by_key(|m| (m.since, m.id));
        rows.into_iter().map(|m| m.person.clone()).collect()
    }

    /// Everything that is in the operator group with the role, marks ignored (for tests of the fail-closed read).
    pub(crate) fn operator_persons_raw(&self) -> Vec<PersonRef> {
        let role = role_operator();
        self.members
            .iter()
            .filter(|m| self.gov.operator_group == Some(m.group) && m.role.as_ref() == Some(&role))
            .map(|m| m.person.clone())
            .collect()
    }

    pub(crate) fn operators(&self) -> OperatorList {
        OperatorList {
            group: self.gov.operator_group,
            persons: self.operator_persons(),
        }
    }

    /// The group "Betreiber", made at the first local command (nobody owns it; everybody sees it).
    fn ensure_operator_group(&mut self, first: &PersonRef) -> ObjectId {
        if let Some(g) = self.gov.operator_group {
            return g;
        }
        let now = self.tick();
        let id = self.fresh();
        let group = Group {
            id,
            reference: reference_of(id),
            revision: Revision(1),
            name: OPERATOR_GROUP_NAME.to_owned(),
            kind: None,
            description: None,
            default_mode: NestingMode::Single,
            visibility: Visibility::Everyone,
            sensitive: false,
            mirror: false,
            created: now,
            created_by: first.clone(),
            updated: now,
        };
        let row = Row {
            group,
            deleted: None,
            rev_at_delete: Revision(1),
        };
        self.groups.insert(id, row);
        self.log(id, HistoryKind::Created);
        self.change(ChangeKind::Group(id));
        let grant = ScopeGrant {
            id: self.fresh(),
            revision: Revision(1),
            admin_group: id,
            scope: Scope::Server,
            granted_by: first.clone(),
            granted: now,
        };
        self.gov.scopes.push(grant);
        self.gov.operator_group = Some(id);
        id
    }

    /// The local command: `person` becomes an operator (after `check_operator_candidate`).
    pub(crate) fn operator_set_local(&mut self, person: &PersonRef) -> Result<()> {
        self.check_person(person)?;
        check_operator_candidate(
            person.art,
            self.people.status(person),
            self.people.supervised(person),
        )?;
        let role = role_operator();
        if let Some(m) = self.members.iter().find(|m| {
            self.gov.operator_group == Some(m.group)
                && &m.person == person
                && m.role.as_ref() == Some(&role)
        }) {
            return Err(Error::AlreadyThere(m.id));
        }
        let g = self.ensure_operator_group(person);
        self.join(g, person, Some(&role_operator()), person, None);
        Ok(())
    }

    /// The local command: `person` is no operator any more.
    pub(crate) fn operator_unset_local(&mut self, person: &PersonRef) -> Result<()> {
        let role = role_operator();
        let found = self
            .members
            .iter()
            .find(|m| {
                self.gov.operator_group == Some(m.group)
                    && &m.person == person
                    && m.role.as_ref() == Some(&role)
            })
            .map(|m| (m.id, m.group));
        let Some((id, g)) = found else {
            return Err(Error::NotFound(Missing::Person(person.id.clone())));
        };
        self.tick();
        self.members.retain(|m| m.id != id);
        self.log(
            g,
            HistoryKind::MemberRemoved {
                person: person.clone(),
            },
        );
        self.change(ChangeKind::Membership { group: g });
        Ok(())
    }

    /// Put an operator in as it is: no checks, no log (old data, a mark that changes later).
    pub(crate) fn operator_insert_raw(&mut self, person: &PersonRef) {
        let g = self.ensure_operator_group(person);
        self.join(g, person, Some(&role_operator()), person, None);
    }
}
