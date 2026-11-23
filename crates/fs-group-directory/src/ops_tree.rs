//! Reading the tree: `nestings`, `ancestors`, `descendants`.
//!
//! Pattern: Repository query (the tree over the confirmed edges; what the asker may not read is left out, also on
//! the way).

use crate::groups_v1::shared::ObjectId;
use crate::groups_v1::{Nesting, NestingSide, Result};
use crate::nesting_logic::reach;
use crate::state::State;

impl State {
    pub(crate) fn nestings(&self, group: ObjectId, side: NestingSide) -> Result<Vec<Nesting>> {
        self.visible(group)?;
        let mut out: Vec<Nesting> = self
            .edges
            .iter()
            .filter(|e| {
                if side == NestingSide::AsChild {
                    e.child == group
                } else {
                    e.parent == group
                }
            })
            .filter(|e| {
                !self.hidden(if side == NestingSide::AsChild {
                    e.parent
                } else {
                    e.child
                })
            })
            .cloned()
            .collect();
        out.sort_by_key(|e| (e.linked, e.id));
        Ok(out)
    }

    pub(crate) fn ancestors(&self, group: ObjectId) -> Result<Vec<ObjectId>> {
        self.visible(group)?;
        Ok(reach(&self.logic_edges(), group, true, &|g| {
            self.readable(g)
        }))
    }

    pub(crate) fn descendants(&self, group: ObjectId) -> Result<Vec<ObjectId>> {
        self.visible(group)?;
        Ok(reach(&self.logic_edges(), group, false, &|g| {
            self.readable(g)
        }))
    }
}
