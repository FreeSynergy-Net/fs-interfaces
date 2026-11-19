//! Who is on the other end of the connection, and what that end may see of the sensitive data of a person.
//!
//! Pattern: Strategy — [`Grants`] is the question "may this recipient see this field now?"; the program that serves the tools
//! answers it from its own store of releases (the access layer, a card of its own). [`DenyAll`] is the default and the test
//! double of "nobody was released". Value objects: [`ConnectionKey`], [`Recipient`], [`Connection`].
//!
//! The recipient comes from the KEY of the connection and from nothing else. The label that a client names itself with
//! (`clientInfo`, `ai:<name>`, anything in `_meta`) is for display and the log; there is no way from it to a [`Recipient`]:
//! the only constructor takes a [`ConnectionKey`], and a key is made by the program that has verified it. Without a key
//! there is no recipient, and every sensitive field is held back.

/// A connection key that the program has verified (a place of a bot, an access key). It names WHO the key stands for
/// (`holder`, as the issuer wrote it); the secret itself never gets here.
pub struct ConnectionKey {
    holder: String,
}

impl ConnectionKey {
    /// A verified key that stands for `holder`. Called by the program after it has checked the key, never with text that the
    /// client sent in a request.
    #[must_use]
    pub fn verified(holder: impl Into<String>) -> Self {
        Self {
            holder: holder.into(),
        }
    }
}

impl std::fmt::Debug for ConnectionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConnectionKey(<hidden>)")
    }
}

/// The one who receives what a call returns. Only a [`ConnectionKey`] makes one.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Recipient(String);

impl Recipient {
    /// The recipient that a verified key stands for.
    #[must_use]
    pub fn of_key(key: &ConnectionKey) -> Self {
        Self(key.holder.clone())
    }

    /// The name the grants know it by.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Recipient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Recipient({})", self.0)
    }
}

/// The connection of one server: with a key it has a recipient, without one it has none.
#[derive(Clone, Debug, Default)]
pub struct Connection {
    recipient: Option<Recipient>,
}

impl Connection {
    /// A connection without a key: no recipient, every sensitive field is held back. This is the default.
    #[must_use]
    pub const fn keyless() -> Self {
        Self { recipient: None }
    }

    /// A connection with a verified key.
    #[must_use]
    pub fn keyed(key: &ConnectionKey) -> Self {
        Self {
            recipient: Some(Recipient::of_key(key)),
        }
    }

    /// The recipient of this connection, if it has a key.
    #[must_use]
    pub const fn recipient(&self) -> Option<&Recipient> {
        self.recipient.as_ref()
    }
}

/// Does the recipient have a release for a sensitive place, NOW? Asked again at every sensitive field of every call: nothing
/// is kept, so a revoked release stops the next call.
pub trait Grants {
    /// `tool` is the name of the tool, `path` the place in its result (`entries.text`, names as the schema writes them).
    fn allows(&self, who: &Recipient, tool: &str, path: &str) -> bool;
}

/// Nobody is released (the default).
#[derive(Clone, Copy, Debug, Default)]
pub struct DenyAll;

impl Grants for DenyAll {
    fn allows(&self, _: &Recipient, _: &str, _: &str) -> bool {
        false
    }
}

/// What a call needs to decide about sensitive fields: the connection, the grants and the name of the tool.
#[derive(Clone, Copy)]
pub struct Access<'a> {
    /// The connection.
    pub connection: &'a Connection,
    /// The question.
    pub grants: &'a dyn Grants,
    /// The tool that runs.
    pub tool: &'a str,
}

impl Access<'_> {
    /// May the place be written into the result now? No recipient, no.
    #[must_use]
    pub fn may_see(&self, path: &str) -> bool {
        self.connection
            .recipient()
            .is_some_and(|who| self.grants.allows(who, self.tool, path))
    }
}
