//! Edges: `request_nesting`, `change_nesting_mode`, `remove_nesting` and the shared `link`.
//!
//! Pattern: Command handlers on the state (one side's `Manage` is enough to weaken or dissolve; strengthening
//! needs both sides, so it becomes a request to the other side).

use crate::groups::shared::{ObjectId, PersonRef, Revision};
use crate::groups::{
    ChangeKind, Error, HistoryKind, Missing, NestOutcome, Nesting, NestingMode, NestingRequest,
    NestingRequestKind, NestingSide, RequestState, Result, Right,
};
use crate::state::State;

impl State {
    /// Put a confirmed edge in (the checks are done) and log it on both groups.
    pub(crate) fn link(
        &mut self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
        by: &PersonRef,
        ok: Option<PersonRef>,
    ) -> Nesting {
        let id = self.fresh();
        let edge = Nesting {
            id,
            revision: Revision(1),
            child,
            parent,
            mode,
            linked: self.now,
            linked_by: by.clone(),
            approved_by: ok,
        };
        self.edges.push(edge.clone());
        self.log(
            child,
            HistoryKind::NestingLinked {
                other: parent,
                mode,
            },
        );
        self.log(parent, HistoryKind::NestingLinked { other: child, mode });
        self.change(ChangeKind::Nesting { child, parent });
        edge
    }

    /// Open a request to the side the asker does not manage.
    pub(crate) fn open_request(
        &mut self,
        child: ObjectId,
        parent: ObjectId,
        kind: NestingRequestKind,
        asks_parent: bool,
    ) -> NestingRequest {
        let (id, mode) = (
            self.fresh(),
            match &kind {
                NestingRequestKind::Link { mode } => *mode,
                NestingRequestKind::ChangeMode { to, .. } => *to,
            },
        );
        let asked = if asks_parent {
            NestingSide::AsParent
        } else {
            NestingSide::AsChild
        };
        let request = NestingRequest {
            id,
            revision: Revision(1),
            child,
            parent,
            kind,
            asked,
            state: RequestState::Pending,
            requested_by: self.asker.clone(),
            requested: self.now,
            answered_by: None,
            answered: None,
        };
        self.requests.push(request.clone());
        self.log(
            child,
            HistoryKind::NestingRequested {
                other: parent,
                mode,
            },
        );
        self.log(parent, HistoryKind::NestingRequested { other: child, mode });
        self.change(ChangeKind::NestingRequest { child, parent });
        request
    }

    /// An open request that concerns this pair (a link) or this edge (a mode change).
    fn open_for(
        &self,
        child: ObjectId,
        parent: ObjectId,
        edge: Option<ObjectId>,
    ) -> Option<ObjectId> {
        self.requests
            .iter()
            .filter(|r| r.state == RequestState::Pending && r.child == child && r.parent == parent)
            .find(|r| match &r.kind {
                NestingRequestKind::Link { .. } => edge.is_none(),
                NestingRequestKind::ChangeMode { nesting, .. } => Some(*nesting) == edge,
            })
            .map(|r| r.id)
    }

    pub(crate) fn request_nesting(
        &mut self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome> {
        self.visible(child)?;
        self.visible(parent)?;
        let (mc, mp) = (
            self.may_ask(child, Right::Manage),
            self.may_ask(parent, Right::Manage),
        );
        if !mc && !mp {
            return Err(Error::Forbidden {
                group: child,
                right: Right::Manage,
            });
        }
        if let Some(e) = self.edge_of(child, parent) {
            return Err(Error::AlreadyThere(e.id));
        }
        if let Some(r) = self.open_for(child, parent, None) {
            return Err(Error::AlreadyThere(r));
        }
        self.check_edge(child, parent, mode, None)?;
        self.tick();
        if mc && mp {
            let me = self.asker.clone();
            return Ok(NestOutcome::Linked(
                self.link(child, parent, mode, &me, None),
            ));
        }
        Ok(NestOutcome::Requested(self.open_request(
            child,
            parent,
            NestingRequestKind::Link { mode },
            mc,
        )))
    }

    /// The confirmed edge a caller names by id; one with a hidden or a deleted end is `NotFound`.
    pub(crate) fn edge_by_id(&self, id: ObjectId) -> Result<Nesting> {
        let missing = || Error::NotFound(Missing::Nesting(id));
        let e = self.edges.iter().find(|e| e.id == id).ok_or_else(missing)?;
        if self.hidden(e.child) || self.hidden(e.parent) {
            return Err(missing());
        }
        Ok(e.clone())
    }

    pub(crate) fn change_nesting_mode(
        &mut self,
        id: ObjectId,
        expected: Revision,
        mode: NestingMode,
    ) -> Result<NestOutcome> {
        let e = self.edge_by_id(id)?;
        let (mc, mp) = (
            self.may_ask(e.child, Right::Manage),
            self.may_ask(e.parent, Right::Manage),
        );
        if !mc && !mp {
            return Err(Error::Forbidden {
                group: e.child,
                right: Right::Manage,
            });
        }
        Self::cas(e.revision, expected)?;
        if e.mode == mode {
            return Err(Error::AlreadyThere(id));
        }
        if let Some(r) = self.open_for(e.child, e.parent, Some(id)) {
            return Err(Error::AlreadyThere(r));
        }
        self.check_edge(e.child, e.parent, mode, Some(id))?;
        self.tick();
        if mode == NestingMode::Single || (mc && mp) {
            return self.set_mode(id, mode).map(NestOutcome::Linked);
        }
        let kind = NestingRequestKind::ChangeMode {
            nesting: id,
            from: e.mode,
            to: mode,
        };
        Ok(NestOutcome::Requested(
            self.open_request(e.child, e.parent, kind, mc),
        ))
    }

    /// Set the mode of an edge (checks done), log it on both groups.
    pub(crate) fn set_mode(&mut self, id: ObjectId, mode: NestingMode) -> Result<Nesting> {
        let e = self
            .edges
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(Error::NotFound(Missing::Nesting(id)))?;
        let from = e.mode;
        e.mode = mode;
        e.revision = Revision(e.revision.0 + 1);
        let edge = e.clone();
        self.log(
            edge.child,
            HistoryKind::NestingModeChanged {
                other: edge.parent,
                from,
                to: mode,
            },
        );
        self.log(
            edge.parent,
            HistoryKind::NestingModeChanged {
                other: edge.child,
                from,
                to: mode,
            },
        );
        self.change(ChangeKind::Nesting {
            child: edge.child,
            parent: edge.parent,
        });
        Ok(edge)
    }

    pub(crate) fn remove_nesting(&mut self, id: ObjectId, expected: Revision) -> Result<()> {
        let e = self.edge_by_id(id)?;
        if !self.may_ask(e.child, Right::Manage) && !self.may_ask(e.parent, Right::Manage) {
            return Err(Error::Forbidden {
                group: e.child,
                right: Right::Manage,
            });
        }
        Self::cas(e.revision, expected)?;
        self.tick();
        self.edges.retain(|x| x.id != id);
        for r in self.requests.iter_mut().filter(|r| {
            r.state == RequestState::Pending && r.child == e.child && r.parent == e.parent
        }) {
            r.state = RequestState::Withdrawn;
            r.revision = Revision(r.revision.0 + 1);
        }
        self.log(e.child, HistoryKind::NestingRemoved { other: e.parent });
        self.log(e.parent, HistoryKind::NestingRemoved { other: e.child });
        self.change(ChangeKind::Nesting {
            child: e.child,
            parent: e.parent,
        });
        Ok(())
    }
}
