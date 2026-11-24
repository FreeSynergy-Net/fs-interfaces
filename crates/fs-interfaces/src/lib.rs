//! FreeSynergy interface contracts (ADR 0045, 0050).
//!
//! The Cap'n Proto schemas live in `schema/`; the Rust code generated from them is committed
//! (ADR 0040) and exposed as one module per schema, so a normal build needs no schema compiler.
//!
//! * [`common_capnp`] - shared types (ids, envelope, change feed, provider-rule types)
//! * [`tasks_capnp`] - tasks (owner `fs-tasks`)
//! * [`messenger_capnp`] - messages (owner `fs-messages`)
//! * [`hub_capnp`] - hub (owner `fs-registry`)
//! * [`people_capnp`] - people and roles (owner `fs-people`)
//! * [`groups_capnp`] - groups, and directories that mirror them (owner `fs-groups`)
//! * [`files_capnp`] - files, and backends that keep the bytes (owner `fs-files`)
//! * [`events_capnp`] - events (owner `fs-calendars`)
//! * [`places_capnp`] - places, and sources of map tiles (owner `fs-maps`)
//! * [`texts_capnp`] - texts (owner `fs-texts`)
//! * [`surveys_capnp`] - surveys and decisions (owner `fs-forms`)
//! * [`history_capnp`] - what happened, across programs (owner `fs-activities`)
//! * [`resources_capnp`] - things, capacity, needs and offers (owner `fs-resources`)
//! * [`whiteboard_capnp`] - boards and their elements (owner `fs-draw`)
//! * [`streaming_capnp`] - live streams and where they go (owner `fs-broadcast`)
//! * [`store_capnp`] - install and update programs from Git sources (owner `fs-packages`)
//! * [`settings_capnp`] - settings of a person and instance defaults, served by one process (owner `fs-settings`)
//! * [`layouts_capnp`] - frontend descriptions, overlays and locks (owner `fs-forms`, draft 0.1, ADR 0096)
//! * [`bind_capnp`] - the host side of binding a frontend description to a program (owner `fs-forms`, draft; reached from `Layouts.bind`)
//! * [`formsource_capnp`] - the read side every module service serves next to its contract, for drawing by a form (owner `fs-forms`, draft; forms in [`forms`])
//! * [`instance_capnp`] - the one window of a program: a second start hands its words to the running one (owner `fs-starter`, draft, ADR 0097)
//! * [`owner`] - where a process takes the local owner from (`LocalOwner`; test sources behind the feature `fake`)
//!
//! The basic types (e-mail, phone, person reference, instant, `TypeCatalog` ...) and their schema live in the crate
//! `fs-types` (ADR 0093), not here, and are not re-exported: a program that needs them depends on `fs-types` itself.
//! The generated modules reference them as `::fs_types::types_capnp`.
//!
//! [`provider`] holds the Rust side of the provider rule: [`Unsupported`] and [`Capabilities`].
//!
//! Pattern: Adapter, the generated modules are the wire form; Null Object, a default method that
//! answers `Unsupported` is the explicit "not possible".

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod common_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod bind_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod instance_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod formsource_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod comments_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod events_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod files_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod groups_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod history_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod hub_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod layouts_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod messenger_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod people_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod places_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod resources_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod settings_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod streaming_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod store_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod surveys_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod tasks_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod whiteboard_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod texts_capnp;

pub mod capabilities;
pub mod forms;
pub mod owner;
pub mod provider;

pub use provider::{Capabilities, Support, UNSUPPORTED_PROBLEM_TYPE, Unsupported};
