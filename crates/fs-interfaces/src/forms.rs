//! The form catalog (draft, PLAN-quelle-und-abgleich section 3): the forms a module can fill and a frontend can draw, one file
//! (`schema/forms.txt`), read here and by nobody else. A form is a named, versioned list of channels (`items`, `label`, `start` ...);
//! a module says which of its fields fill which channel, a component says which forms it draws. This file is the shared vocabulary of
//! the contract `FormSource` (`formsource.capnp`), which carries the records, not the form names.
//!
//! The reader has two parts: [`parse`] (in `parse.rs`) reads and checks a text strictly (a bad line is an error with its line number, never skipped),
//! and the lookups ([`catalog`], [`find`]) work on the built-in file. Pattern: Registry (a fixed table), like `capabilities`.

mod catalog;
mod parse;
mod types;

pub use catalog::{FORM_SOURCE_LEVEL, SOURCE, catalog, find, find_key, key, parse_key};
pub use parse::{ErrorKind, ParseError, parse};
pub use types::{Art, Channel, Count, Form, Status, Version, parse_version};
