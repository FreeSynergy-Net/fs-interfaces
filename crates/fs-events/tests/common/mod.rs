//! Helpers shared by the tests.
#![allow(dead_code)]

use fs_calendars_testdata::draft::{
    Appointment, Appointments, Instance, LocalDateTime, ObjectId, Utc, Window, Zone,
};
use fs_calendars_testdata::{Sample, sample};

/// Milliseconds of a UTC time.
pub fn utc(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Utc {
    let n = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap();
    Utc::from_millis(n.and_utc().timestamp_millis())
}

pub fn loaded() -> (Sample, fs_calendars_testdata::FakeAppointments) {
    let s = sample();
    let f = s.fake();
    (s, f)
}

pub async fn window(f: &impl Appointments, from: Utc, to: Utc) -> Window {
    f.window(from, to, &utc_zone(), &[]).await.unwrap()
}

pub fn utc_zone() -> Zone {
    fs_calendars_testdata::zone::iana("UTC")
}

pub async fn window_in(f: &impl Appointments, from: Utc, to: Utc, zone: &str) -> Window {
    f.window(from, to, &fs_calendars_testdata::zone::iana(zone), &[])
        .await
        .unwrap()
}

pub fn of(w: &Window, a: &Appointment) -> Vec<Instance> {
    w.instances
        .iter()
        .filter(|i| i.appointment == a.id)
        .cloned()
        .collect()
}

pub fn id_of(s: &Sample, slug: &str) -> ObjectId {
    s.get(slug).id
}

pub fn rid(a: &Appointment, i: usize) -> LocalDateTime {
    a.overrides[i].recurrence_id
}
