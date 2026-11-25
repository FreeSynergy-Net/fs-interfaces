//! Who asks: the `Context` of one connection.
//!
//! Pattern: Context Object - who, over which way, with which program, on which device; built ONLY on the module side from
//! the connection. Deviation: never a parameter of the caller; what the program says about itself (`Claimed`, from
//! `introduce`) counts for history and undo, never for rights.

use fs_types::PersonRef;

/// Pattern: Context Object - who asks, for ONE connection until it ends.
///
/// Honest limits: `#[non_exhaustive]` only stops a struct literal outside this crate. The fields are `pub` (readable), the
/// type is `Clone`, and `from_connection` is `pub`, so the type itself does NOT stop a caller from building a context. The
/// protection is architecture: modules get a `&Context` only from `fs_rpc::Guarded`, which builds it once per connection in
/// `bootstrap` from the system's answer; no contract method takes a context from a call argument.
/// A module never keeps a "current user" of its own (Stufe 4 rule, S3 C2).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Context {
    /// How the request came in (the system says who).
    pub caller: Caller,
    /// A person or a bot, from the connection.
    pub actor: Actor,
    /// The person this connection acts for, from the connection (never from an argument).
    pub person: PersonRef,
    /// What the program says about itself (`introduce`); empty until then. Never a right.
    pub claimed: Claimed,
}

impl Context {
    /// The context of a new connection: `claimed` is empty. Only the layer that owns the connection calls this.
    #[must_use]
    pub fn from_connection(caller: Caller, actor: Actor, person: PersonRef) -> Self {
        Self {
            caller,
            actor,
            person,
            claimed: Claimed::default(),
        }
    }

    /// The same connection after `introduce`: replaces `claimed` only, no right changes with it.
    #[must_use]
    pub fn with_claimed(mut self, claimed: Claimed) -> Self {
        self.claimed = claimed;
        self
    }
}

/// Pattern: Value Object - what a program says about itself (`introduce`): given, not proven.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Claimed {
    /// `AppStream` id of the program; empty allowed.
    pub program: String,
    /// Instance of the program (process/window); empty allowed.
    pub instance: String,
    /// Device key (later the iroh `NodeId`), never a display name; empty allowed.
    pub device: String,
}

impl Claimed {
    /// A claim of program, instance and device.
    #[must_use]
    pub fn new(program: &str, instance: &str, device: &str) -> Self {
        Self {
            program: program.into(),
            instance: instance.into(),
            device: device.into(),
        }
    }
}

/// Pattern: State - how the request came in; open for origins without a living caller.
/// Deviation: no branch "from the server" as the only source of rights; a server is just another participant (user 65, mesh).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Caller {
    /// A living connection; the system says who (fs-rpc `transport/peer.rs`: uid/SID). Single place and device.
    Connection {
        /// The system id (uid or SID) as text.
        system_id: String,
        /// The system name, where known.
        system_name: Option<String>,
    },
    // Appended later, not now: Token { issuer, subject } (OIDC), Imported { origin, via } (S1 E4), Signed { by, key } (mesh, 0067).
}

/// Pattern: Value Object - a person or a bot. Same content as fs-settings `Actor`; fs-settings switches to this one later (card).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Actor {
    /// A human.
    Person,
    /// A program acting on its own.
    Bot,
}
