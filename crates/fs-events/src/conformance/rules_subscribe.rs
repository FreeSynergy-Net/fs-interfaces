//! Rules for subscriptions and accounts: the address rule (SSRF), read-only, fetching, secrecy of the address.
//!
//! Pattern: Template method (rules as plain checks collected in the report).

use super::cx::{Cx, invalid_at, is_invalid, read_only_of};
use super::fixture::Asker;
use crate::draft::{
    Answer, AppointmentDraft, AppointmentPatch, BasicValue, CalendarPatch, CalendarSource,
    FetchState, OriginChannel, Reference, Refresh, SeriesScope, Trash,
};
use crate::zone::reference;

/// Addresses a fetch must refuse (R7/K4): scheme, credentials, every non-global range, also embedded forms.
const REFUSED: [&str; 17] = [
    "http://kalender.example.org/a.ics",
    "ftp://kalender.example.org/a.ics",
    "file:///etc/passwd",
    "https://user:secret@kalender.example.org/a.ics",
    "https://127.0.0.1/a.ics",
    "https://10.1.2.3/a.ics",
    "https://172.16.0.9/a.ics",
    "https://192.168.1.1/a.ics",
    "https://169.254.169.254/latest/meta-data/",
    "https://100.64.0.1/a.ics",
    "https://0.0.0.0/a.ics",
    "https://[::1]/a.ics",
    "https://[fc00::1]/a.ics",
    "https://[fe80::1]/a.ics",
    "https://[::ffff:10.0.0.1]/a.ics",
    "https://[64:ff9b::a00:1]/a.ics",
    "https://[2002:a00:1::]/a.ics",
];

const GOOD: &str = "https://kalender.example.org/zzconf-ok.ics";

pub(super) async fn address(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    for text in REFUSED {
        let Ok(url) = Reference::parse(text) else {
            continue;
        };
        let r = dir
            .subscribe_calendar("Zzconf feed", None, &url, Refresh::Manual)
            .await;
        let ok = r.as_ref().err().is_some_and(|e| invalid_at("url", None)(e));
        cx.ensure(
            ok,
            &format!("subscribe_calendar: {text} is Invalid at `url`"),
        );
        let leak = r.as_ref().err().is_some_and(|e| {
            format!("{e:?}").contains("169.254") || format!("{e:?}").contains("secret")
        });
        cx.ensure(
            !leak,
            "subscribe_calendar: the problem never names the address",
        );
    }
    let good = reference(GOOD);
    let r = dir
        .subscribe_calendar("Zzconf feed", None, &good, Refresh::Manual)
        .await;
    let Ok(c) = r else {
        cx.ensure(
            false,
            "subscribe_calendar: a public https address is accepted",
        );
        return;
    };
    cx.ensure(
        matches!(&c.source, CalendarSource::Subscribed(s) if s.url.as_ref() == Some(&good) && s.state == FetchState::Never && s.refresh == Refresh::Manual),
        "subscribe_calendar: a subscription that was never fetched, with its address for the owner",
    );
    let web = reference("webcal://kalender.example.org/zzconf-web.ics");
    cx.ensure(
        dir.subscribe_calendar("Zzconf webcal", None, &web, Refresh::Manual)
            .await
            .is_ok(),
        "subscribe_calendar: webcal: is allowed",
    );
    let r = dir
        .subscribe_calendar("Zzconf rate", None, &good, Refresh::Every { seconds: 5 })
        .await;
    cx.fails(
        &r,
        "subscribe_calendar: a repeat time that is too short is Invalid",
        invalid_at("refresh", None),
    );
    let r = dir
        .subscribe_calendar(" ", None, &good, Refresh::Manual)
        .await;
    cx.fails(
        &r,
        "subscribe_calendar: an empty name is Invalid",
        invalid_at("name", None),
    );
    let own = dir
        .update_calendar(
            c.id,
            c.revision,
            CalendarPatch {
                url: Some(reference(REFUSED[4])),
                ..CalendarPatch::default()
            },
        )
        .await;
    cx.fails(
        &own,
        "update_calendar: a new address is checked like the first (Invalid at `url`)",
        invalid_at("url", None),
    );
    let moved = reference("https://kalender.example.org/zzconf-moved.ics");
    let patch = CalendarPatch {
        url: Some(moved.clone()),
        ..CalendarPatch::default()
    };
    let Ok(c2) = dir.update_calendar(c.id, c.revision, patch).await else {
        cx.ensure(false, "update_calendar: a new public address is accepted");
        return;
    };
    cx.ensure(
        matches!(&c2.source, CalendarSource::Subscribed(s) if s.url.as_ref() == Some(&moved)),
        "update_calendar: changes the address",
    );
    // The address is a secret: never in the log.
    let log = dir
        .history(c.id, None, 100)
        .await
        .map(|p| format!("{:?}", p.items))
        .unwrap_or_default();
    cx.ensure(
        !log.contains("zzconf-moved") && !log.contains("zzconf-ok"),
        "history: the address is never in the log",
    );
    // Only the owner sees the address.
    if cx.fx.switch_asker(Asker::Other).await {
        let seen = cx.listed(c.id).await;
        cx.ensure(
            seen.as_ref().is_some_and(
                |x| matches!(&x.source, CalendarSource::Subscribed(s) if s.url.is_none()),
            ),
            "calendars: someone without calendar.update does not see the address",
        );
        cx.fx.switch_asker(Asker::Owner).await;
    } else {
        cx.skip(
            "calendars: the address is hidden from others (the fixture cannot switch the asker)",
        );
    }
    let Some(own_cal) = cx.calendar("not-a-feed").await else {
        return;
    };
    let r = dir.refresh_calendar(own_cal.id).await;
    cx.fails(
        &r,
        "refresh_calendar: an own calendar is Invalid",
        is_invalid,
    );
}

pub(super) async fn feed(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let address = "https://kalender.example.org/zzconf-feed.ics";
    if !cx
        .fx
        .set_feed(
            address,
            &[("zzconf-1", "Zzconf Eins"), ("zzconf-2", "Zzconf Zwei")],
        )
        .await
    {
        cx.skip("feed: the fixture cannot say what a feed delivers");
        return;
    }
    let Ok(c) = dir
        .subscribe_calendar("Zzconf feed 2", None, &reference(address), Refresh::Manual)
        .await
    else {
        cx.ensure(false, "subscribe_calendar: answers");
        return;
    };
    let Ok(r) = dir.refresh_calendar(c.id).await else {
        cx.ensure(false, "refresh_calendar: answers");
        return;
    };
    cx.ensure(matches!(&r.calendar.source, CalendarSource::Subscribed(s) if matches!(s.state, FetchState::Fresh { .. })), "refresh_calendar: the state is Fresh");
    cx.ensure(
        r.report.created.len() == 2,
        "refresh_calendar: the appointments of the feed come in",
    );
    let items = cx.all(&[c.id], Trash::Exclude).await;
    cx.ensure(
        items.len() == 2,
        "refresh_calendar: they are in the calendar",
    );
    let Some(first) = items.first() else {
        return;
    };
    let Some(a) = cx.fresh(first.id).await else {
        return;
    };
    let o = a.origin.clone();
    cx.ensure(
        o.as_ref().is_some_and(|o| {
            o.channel == OriginChannel::Subscription
                && !o.source.canonical().contains("zzconf-feed")
        }),
        "Origin: channel Subscription, source = the calendar, never the feed address",
    );
    let nf = read_only_of(c.id);
    let draft = AppointmentDraft {
        calendar: c.id,
        uid: None,
        title: "Zzconf mine".to_owned(),
        when: Cx::when(2),
        rest: AppointmentPatch::default(),
    };
    let r = dir.create(draft).await;
    cx.fails(&r, "create in a subscription is ReadOnly", &nf);
    let patch = AppointmentPatch {
        title: Some("x".to_owned()),
        ..AppointmentPatch::default()
    };
    let r = dir
        .update(a.id, a.revision, patch, SeriesScope::Whole)
        .await;
    cx.fails(&r, "update in a subscription is ReadOnly", &nf);
    let r = dir.delete(a.id, a.revision, SeriesScope::Whole).await;
    cx.fails(&r, "delete in a subscription is ReadOnly", &nf);
    let r = dir.invite(a.id, a.revision, &[], SeriesScope::Whole).await;
    cx.fails(&r, "invite in a subscription is ReadOnly", &nf);
    let me = c.owner.clone();
    let r = dir
        .respond(
            a.id,
            a.revision,
            &me,
            Answer::Accept,
            None,
            None,
            SeriesScope::Whole,
        )
        .await;
    cx.fails(&r, "respond in a subscription is ReadOnly", &nf);
    // The source drops one and renames the other.
    cx.fx
        .set_feed(address, &[("zzconf-1", "Zzconf Eins (neu)")])
        .await;
    let Ok(r) = dir.refresh_calendar(c.id).await else {
        cx.ensure(false, "refresh_calendar: answers again");
        return;
    };
    cx.ensure(
        r.removed.len() == 1 && r.updated.len() == 1,
        "refresh_calendar: what the source dropped is removed, what it changed is updated",
    );
    let gone = cx.all(&[c.id], Trash::Only).await;
    cx.ensure(
        gone.len() == 1 && gone.iter().all(|s| r.removed.contains(&s.id)),
        "refresh_calendar: removed ones go to the trash, not for good",
    );
    if let Some(g) = gone.first() {
        let r = dir.restore(g.id, SeriesScope::Whole).await;
        cx.fails(
            &r,
            "restore in a subscription is ReadOnly (the source decides)",
            read_only_of(c.id),
        );
        cx.ensure(
            dir.purge(g.id).await.is_ok(),
            "purge of a subscription appointment in the trash is allowed",
        );
    }
    // A feed that cannot be fetched is a state, not an error; the old appointments stay.
    let other = "https://kalender.example.org/zzconf-down.ics";
    let Ok(d) = dir
        .subscribe_calendar("Zzconf down", None, &reference(other), Refresh::Manual)
        .await
    else {
        return;
    };
    for want in 1..=2_u32 {
        let r = dir.refresh_calendar(d.id).await;
        let ok = r.as_ref().is_ok_and(|x| {
            matches!(&x.calendar.source, CalendarSource::Subscribed(s)
                if matches!(&s.state, FetchState::Failed { attempts, reason, .. } if *attempts == want && !reason.contains("example")))
        });
        cx.ensure(ok, "refresh_calendar: a failed fetch is Ok with FetchState::Failed and counts the attempts");
    }
}
