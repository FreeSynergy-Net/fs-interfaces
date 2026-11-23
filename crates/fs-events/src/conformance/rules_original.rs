//! Rule: a rule kept as written (`Recurrence::Original`) is received, never made (the trait gives no way to make one).
//!
//! Pattern: Template method (plain checks collected in the report).

use super::cx::Cx;
use crate::draft::{AppointmentDraft, AppointmentPatch, Error, Recurrence};

/// `create` with an `Original` recurrence is `Unsupported(event.create)`.
pub(super) async fn original(cx: &mut Cx<'_>) {
    let Some(c) = cx.calendar("original").await else {
        return;
    };
    let draft = AppointmentDraft {
        calendar: c.id,
        uid: None,
        title: "Zzconf original".to_owned(),
        when: Cx::when(8),
        rest: AppointmentPatch {
            recurrence: Some(vec![Recurrence::Original(
                "FREQ=YEARLY;BYWEEKNO=21".to_owned(),
            )]),
            ..AppointmentPatch::default()
        },
    };
    let r = cx.dir.create(draft).await;
    cx.fails(
        &r,
        "create: a rule kept as written (Original) cannot be made: Unsupported(event.create)",
        |e| matches!(e, Error::Unsupported { capability, .. } if capability == "event.create"),
    );
    let Some(a) = cx.appointment(c.id, "plain", 9).await else {
        return;
    };
    let change = AppointmentPatch {
        recurrence: Some(vec![Recurrence::Original(
            "FREQ=YEARLY;BYWEEKNO=21".to_owned(),
        )]),
        ..AppointmentPatch::default()
    };
    let r = cx
        .dir
        .update(a.id, a.revision, change, crate::draft::SeriesScope::Whole)
        .await;
    cx.fails(
        &r,
        "update: a rule kept as written (Original) cannot be set: Unsupported(event.update)",
        |e| matches!(e, Error::Unsupported { capability, .. } if capability == "event.update"),
    );
}
