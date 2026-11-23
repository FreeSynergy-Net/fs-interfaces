//! The rules a mutant of the fake can break (one at a time).

/// The rule that is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutant {
    /// Nothing is broken: a plain forward.
    None,
    /// `purge` deletes what is kept (the retention does not count).
    PurgeIgnoresHold,
    /// `purge` answers `Ok` and deletes nothing.
    PurgeDoesNothing,
    /// A person who does not share is `Unknown` (tells nothing about a calendar, but breaks the rule).
    NotSharedAsUnknown,
    /// An unknown participant is `Busy` with nothing (free: a guess).
    UnknownGuessedFree,
    /// Writing into a calendar in the trash is `Invalid` instead of `NotFound(calendar)`.
    TrashedWriteInvalid,
    /// A subscription answers `Forbidden` instead of `ReadOnly`.
    ReadOnlyAsForbidden,
    /// The cloud-metadata address is accepted as a feed.
    FeedAcceptsMetadata,
    /// A retention with an end shows as `Manual` (the end is lost, K8).
    HeldEndLost,
    /// An AI as owner is `Invalid` instead of `Forbidden`.
    AiOwnerInvalid,
    /// `update_calendar` does not look at the revision.
    NoCompareAndSet,
    /// A hidden calendar is `Forbidden` for an AI instead of `NotFound`.
    HiddenForbidden,
    /// The feed address is the `Origin.source` of a subscribed appointment.
    FeedInOrigin,
    /// `delete_calendar` does not count the appointments it took along.
    CountLost,
    /// A feed address that contains this text is accepted (scheme, credentials, embedded IPv6 ...).
    FeedAccepts(&'static str),
    /// `purge` of a living object answers `Ok` instead of `Invalid`.
    PurgeLivingOk,
    /// A retention without end shows as `Manual`.
    HeldNoneAsManual,
    /// `restore_calendar` does not bring the appointments back that went with it.
    RestoreNoCascade,
    /// The address of a subscription is shown to someone without `calendar.update`.
    AddressShown,
    /// An AI's `delete_calendar` is `Invalid` instead of `Forbidden`.
    AiDeleteInvalid,
    /// An AI's `subscribe_calendar` is `Invalid` instead of `Forbidden`.
    AiSubscribeInvalid,
    /// An AI's `purge` is `Invalid` instead of `Forbidden`.
    AiPurgeInvalid,
    /// `restore` in a subscription is `Forbidden` instead of `ReadOnly`.
    RestoreSubscribedForbidden,
    /// A retention of an appointment does not keep its trashed calendar (it goes at the calendar's deadline).
    DeadlineIgnoresHold,
    /// A kept calendar loses its trashed appointments (the period takes them although the calendar is kept).
    HeldCalendarLosesAppointments,
    /// A calendar shown as `Held` cannot be restored (`NotFound`), as the local provider did before.
    HeldCalendarNotRestorable,
    /// A recurrence given as `Original` is accepted at `create`.
    OriginalAccepted,
}

pub fn is(m: Mutant, want: Mutant) -> bool {
    m == want
}
