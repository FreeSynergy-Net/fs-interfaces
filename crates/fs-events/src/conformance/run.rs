//! `run_conformance`: all rules, in an order that keeps clock moves for last.
//!
//! Pattern: Template method.

use super::cx::{Cx, Report};
use super::fixture::ConformanceFixture;
use super::{
    rules_ai, rules_calendars, rules_deadline, rules_freebusy, rules_original, rules_purge,
    rules_subscribe,
};

/// Run every rule against the provider of the fixture. The report names the failed rules; `skipped` names those a
/// fixture could not set up.
pub async fn run_conformance(fx: &dyn ConformanceFixture) -> Report {
    let mut cx = Cx::new(fx);
    let cal = [
        "calendar.create",
        "calendar.update",
        "calendar.delete",
        "calendar.restore",
    ];
    let purge = [
        "calendar.create",
        "event.create",
        "event.delete",
        "trash.purge",
    ];
    if cx.offers(&cal) {
        rules_calendars::manage(&mut cx).await;
        rules_calendars::cascade(&mut cx).await;
    } else {
        cx.skip("calendars: the provider does not offer calendar.update/delete/restore");
    }
    if cx.offers(&["calendar.create", "event.create", "event.update"]) {
        rules_original::original(&mut cx).await;
    }
    if cx.offers(&["calendar.subscribe"]) {
        rules_subscribe::address(&mut cx).await;
        if cx.offers(&["calendar.refresh"]) {
            rules_subscribe::feed(&mut cx).await;
        }
    } else {
        cx.skip("subscriptions: the provider does not offer calendar.subscribe");
    }
    if cx.offers(&["event.freebusy"]) {
        rules_freebusy::free_busy(&mut cx).await;
    } else {
        cx.skip("free_busy: the provider does not offer event.freebusy");
    }
    if cx.offers(&cal) {
        rules_ai::ai(&mut cx).await;
    }
    if cx.offers(&purge) {
        rules_purge::purge(&mut cx).await;
        rules_purge::retention(&mut cx).await;
        // The clock moves last: later rules would see a different "now".
        rules_purge::period(&mut cx).await;
        rules_deadline::kept_calendar(&mut cx).await;
        rules_deadline::kept_calendar_keeps_appointments(&mut cx).await;
    } else {
        cx.skip("purge: the provider does not offer trash.purge");
    }
    cx.report
}
