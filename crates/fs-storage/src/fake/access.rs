//! Who may do what: owner of the entry, group owners and members, managers (structure only), the operator (reads all),
//! bots (nothing).
//!
//! Pattern: Policy (one function per question; no rights model of its own, ADR 0095).

use super::state::{Fault, Node, World};
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
    pub(super) fn is_bot(&self, a: &Actor) -> bool {
        a.fault != Some(Fault::BotAllowed) && self.bots.iter().any(|p| p.id == a.who.id)
    }

    pub(super) fn is_operator(&self, a: &Actor) -> bool {
        a.fault != Some(Fault::OperatorBlocked) && self.operators.iter().any(|p| p.id == a.who.id)
    }

    pub(super) fn is_admin(&self, a: &Actor) -> bool {
        self.admins.iter().any(|p| p.id == a.who.id)
    }

    /// Own entries: everything. A group's entries: owners everything, members read only. A manager (admin level (a)/(b),
    /// no server access) sees the structure of a group, never a private folder. The operator (server access) reads
    /// everything without an intervention (user decision 16, said openly by the interface). Foreign entries are changed
    /// only through an intervention with a reason (16.3: an ordered procedure and traceability, never a technical lock
    /// against root), which is no part of this contract. A log of the operator's reads is information only: it protects
    /// nothing (whoever is root can read past it). A bot has no right and never owns anything. Everyone else: nothing.
    pub(super) fn allowed(&self, a: &Actor, owner: &PersonRef, mode: Mode) -> bool {
        if a.fault == Some(Fault::IgnoreRights)
            || (a.fault == Some(Fault::AdminReadsAll) && self.is_admin(a))
        {
            return true;
        }
        if self.bots.iter().any(|p| p.id == a.who.id) && a.fault != Some(Fault::BotAllowed) {
            return false;
        }
        if a.who.id == owner.id
            || ((mode != Mode::Write || a.fault == Some(Fault::OperatorChanges))
                && self.is_operator(a))
        {
            return true;
        }
        let Some(g) = self.groups.iter().find(|g| g.who.id == owner.id) else {
            return false;
        };
        let member = g.owners.iter().chain(&g.members).any(|p| p.id == a.who.id);
        let owner_of = g.owners.iter().any(|p| p.id == a.who.id);
        match mode {
            Mode::Write => owner_of || (a.fault == Some(Fault::ManagerChanges) && self.is_admin(a)),
            Mode::Read => member,
            Mode::Structure => member || self.is_admin(a),
        }
    }

    /// The finer mutants of the change rights: write exceptions at roots and links (never set in normal use).
    pub(super) fn exception(&self, a: &Actor, n: &Node, mode: Mode) -> bool {
        if mode != Mode::Write {
            return false;
        }
        let root = n.parent.is_none();
        let group_root = root && self.groups.iter().any(|g| g.who.id == n.entry.owner.id);
        match a.fault {
            Some(Fault::ManagerGroupRoot) => group_root && self.is_admin(a),
            Some(Fault::OperatorRoot) => root && self.is_operator(a),
            Some(Fault::OperatorLink) => {
                n.entry.kind == crate::entry::Kind::Link && self.is_operator(a)
            }
            _ => false,
        }
    }
}
