//! Who may do what: owner of the entry, group owners and members, admins (structure only), bots (nothing).
//!
//! Pattern: Policy (one function per question; no rights model of its own, ADR 0095).

use super::state::{Fault, World};
use fs_types::PersonRef;

pub(super) struct Actor<'a> {
    pub who: &'a PersonRef,
    pub fault: Option<Fault>,
}

/// What a call needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Names, sizes, times (`list`, `stat`).
    Structure,
    /// The content (`open`).
    Read,
    /// Any change.
    Write,
}

impl World {
    pub(super) fn is_admin(&self, a: &Actor) -> bool {
        self.admins.iter().any(|p| p.id == a.who.id)
    }

    /// Own entries: everything. A group's entries: owners everything, members read only; an admin sees the structure of
    /// a group, never a private folder. A bot has no right at all, and never owns anything (the AI is an actor, not an
    /// owner). Everyone else: nothing.
    pub(super) fn allowed(&self, a: &Actor, owner: &PersonRef, mode: Mode) -> bool {
        if a.fault == Some(Fault::IgnoreRights)
            || (a.fault == Some(Fault::AdminReadsAll) && self.is_admin(a))
        {
            return true;
        }
        if self.bots.iter().any(|p| p.id == a.who.id) && a.fault != Some(Fault::BotAllowed) {
            return false;
        }
        if a.who.id == owner.id {
            return true;
        }
        let Some(g) = self.groups.iter().find(|g| g.who.id == owner.id) else {
            return false;
        };
        let member = g.owners.iter().chain(&g.members).any(|p| p.id == a.who.id);
        let owner_of = g.owners.iter().any(|p| p.id == a.who.id);
        match mode {
            Mode::Write => owner_of,
            Mode::Read => member,
            Mode::Structure => member || self.is_admin(a),
        }
    }
}
