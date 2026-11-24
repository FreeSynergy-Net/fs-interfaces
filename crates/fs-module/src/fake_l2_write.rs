//! The writes of level 2 on the shared state: `invokeIf` (compare-and-set) and `revert` (undo as a new change).
//!
//! Pattern: Fake Object (see `fake_l2`; this file only holds the two write paths so each file stays small).

use crate::fake::{Refusal, refuse};
use crate::fake_l2::{Outcome, Shared, Who};

impl Shared {
    /// `invokeIf`: compare-and-set, then the command of level 1, then the log.
    pub(crate) fn write_if(
        &self,
        run_command: impl FnOnce() -> Result<(String, &'static str), Refusal>,
        record: &str,
        args: &[(String, String)],
        expect: &str,
        who: &Who,
        step: (&str, &str),
    ) -> Outcome {
        let stale = !expect.is_empty()
            && self
                .rev_of(record)
                .is_some_and(|rev| rev.to_string() != expect);
        if stale {
            let cur = self.image(record);
            let fields = args
                .iter()
                .filter_map(|(p, mine)| {
                    let c = cur.as_ref()?;
                    let now = match p.as_str() {
                        "title" => &c.title,
                        "column" => &c.column,
                        _ => return None,
                    };
                    Some((p.clone(), now.clone(), mine.clone()))
                })
                .collect();
            return Outcome::Clash {
                by: self.last_of(record),
                fields,
            };
        }
        let before = self.image(record);
        let (id, text) = match run_command() {
            Ok(done) => done,
            Err(why) => return Outcome::Refused(why),
        };
        let object = if id.is_empty() { record } else { id.as_str() };
        let after = self.image(object);
        let kind = match (&before, &after) {
            (None, _) => "Create",
            (_, None) => "Delete",
            _ => "Update",
        };
        Outcome::Done(self.commit(
            object,
            who,
            kind,
            0,
            (false, step.0, step.1),
            text,
            (before, after),
        ))
    }

    /// `revert`: the before-image of the change as a new `Undo`, if the record is still as the change left it.
    pub(crate) fn revert(&self, id: u128, who: &Who, step: (&str, &str)) -> Outcome {
        let Some(e) = self.find(id) else {
            return Outcome::Refused(refuse("unknown-change", 404, "no such change"));
        };
        if let Some(last) = self.last_of(&e.object).filter(|l| l.id != e.id) {
            let (now, mine) = (self.image(&e.object), e.before.clone());
            let mut fields = Vec::new();
            if let (Some(n), Some(m)) = (now, mine) {
                for (p, a, b) in [("title", n.title, m.title), ("column", n.column, m.column)] {
                    if a != b {
                        fields.push((p.to_owned(), a, b));
                    }
                }
            }
            return Outcome::Clash {
                by: Some(last),
                fields,
            };
        }
        self.put(&e.object, e.before.as_ref());
        Outcome::Done(self.commit(
            &e.object,
            who,
            "Undo",
            e.id,
            (true, step.0, step.1),
            "undone",
            (e.after.clone(), e.before.clone()),
        ))
    }
}
