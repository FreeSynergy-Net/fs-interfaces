//! Requests between groups: `nesting_requests`, `answer_nesting_request`, `withdraw_nesting_request`.
//!
//! Pattern: State (the life of a request: only `Pending` can be answered or withdrawn) + Command handlers.

use crate::groups::shared::{ObjectId, Revision};
use crate::groups::{
    ChangeKind, Error, HistoryKind, Missing, NestingRequest, NestingRequestKind, NestingSide,
    RequestAnswer, RequestSide, RequestState, Result, Right,
};
use crate::state::State;

impl State {
    pub(crate) fn nesting_requests(
        &self,
        group: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>> {
        self.visible(group)?;
        self.need(group, Right::Manage)?;
        let mut out: Vec<NestingRequest> = self
            .requests
            .iter()
            .filter(|r| r.child == group || r.parent == group)
            .filter(|r| !self.hidden(r.child) && !self.hidden(r.parent))
            .filter(|r| {
                let asked_here = (r.asked == NestingSide::AsChild) == (r.child == group);
                asked_here == (side == RequestSide::Incoming)
            })
            .cloned()
            .collect();
        out.sort_by_key(|r| (r.requested, r.id));
        Ok(out)
    }

    /// The request by id; one that names a hidden or deleted group is `NotFound`.
    fn request_by_id(&self, id: ObjectId) -> Result<NestingRequest> {
        let missing = || Error::NotFound(Missing::NestingRequest(id));
        let r = self
            .requests
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(missing)?;
        if self.visible(r.child).is_err() || self.visible(r.parent).is_err() {
            return Err(missing());
        }
        Ok(r.clone())
    }

    fn close_request(
        &mut self,
        id: ObjectId,
        state: RequestState,
        answered: bool,
    ) -> Result<NestingRequest> {
        let (me, now) = (self.asker.clone(), self.now);
        let r = self
            .requests
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(Error::NotFound(Missing::NestingRequest(id)))?;
        r.state = state;
        r.revision = Revision(r.revision.0 + 1);
        if answered {
            r.answered_by = Some(me);
            r.answered = Some(now);
        }
        let done = r.clone();
        self.log(
            done.child,
            HistoryKind::NestingAnswered {
                other: done.parent,
                state,
            },
        );
        self.log(
            done.parent,
            HistoryKind::NestingAnswered {
                other: done.child,
                state,
            },
        );
        self.change(ChangeKind::NestingRequest {
            child: done.child,
            parent: done.parent,
        });
        Ok(done)
    }

    pub(crate) fn answer_nesting_request(
        &mut self,
        id: ObjectId,
        expected: Revision,
        answer: RequestAnswer,
    ) -> Result<NestingRequest> {
        let r = self.request_by_id(id)?;
        let side = if r.asked == NestingSide::AsChild {
            r.child
        } else {
            r.parent
        };
        self.need(side, Right::Manage)?;
        Self::cas(r.revision, expected)?;
        if r.state != RequestState::Pending {
            return Err(Error::RequestClosed(r.state));
        }
        self.tick();
        if answer == RequestAnswer::Decline {
            return self.close_request(id, RequestState::Declined, true);
        }
        let me = self.asker.clone();
        match r.kind {
            NestingRequestKind::Link { mode } => {
                if let Some(e) = self.edge_of(r.child, r.parent) {
                    return Err(Error::AlreadyThere(e.id));
                }
                self.check_edge(r.child, r.parent, mode, None)?;
                self.link(r.child, r.parent, mode, &r.requested_by, Some(me));
            }
            NestingRequestKind::ChangeMode { nesting, from, to } => {
                let edge = self.edges.iter().find(|e| e.id == nesting).cloned();
                let edge = edge.ok_or(Error::NotFound(Missing::Nesting(nesting)))?;
                if edge.mode != from {
                    return Err(Error::Changed {
                        current: edge.revision,
                    });
                }
                self.check_edge(r.child, r.parent, to, Some(nesting))?;
                self.set_mode(nesting, to)?;
            }
        }
        self.close_request(id, RequestState::Approved, true)
    }

    pub(crate) fn withdraw_nesting_request(
        &mut self,
        id: ObjectId,
        expected: Revision,
    ) -> Result<NestingRequest> {
        let r = self.request_by_id(id)?;
        let asking_side = if r.asked == NestingSide::AsChild {
            r.parent
        } else {
            r.child
        };
        if r.requested_by != self.asker && !self.may_ask(asking_side, Right::Manage) {
            return Err(Error::Forbidden {
                group: asking_side,
                right: Right::Manage,
            });
        }
        Self::cas(r.revision, expected)?;
        if r.state != RequestState::Pending {
            return Err(Error::RequestClosed(r.state));
        }
        self.tick();
        self.close_request(id, RequestState::Withdrawn, false)
    }
}
