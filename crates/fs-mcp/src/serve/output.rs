//! The result of a call as JSON, in the form of the capnp-JSON codec and of the `outputSchema` of the tool.
//!
//! Pattern: Visitor — [`JsonOutput`] is the `Output` the generated code walks a result into. It follows the output
//! schema in step, because the walk does not say how wide an integer is: a 64-bit integer is written as a string
//! (`x-fs-int`), as the schema says. `Data` is base64 (the rule of the contract, not the codec default of a byte list), a
//! float that is not finite is one of three strings, an enumerant is its name. What cannot be shown (an enumerant this
//! version does not know, a capability, a value the generator could not read) is not shown as a number or dropped: the
//! output is marked [`JsonOutput::problem`] and the call is an error.

use capnp_compiler::cligen::model::kebab;
use capnp_compiler::cligen::rt::{Output, Value as V};
use serde_json::{Map, Number, Value};

use super::access::Access;
use super::{base64, schema};

/// The schema of a place that the output schema does not know: it declares nothing.
static UNKNOWN: Value = Value::Null;

enum Frame {
    Object(Map<String, Value>),
    List(Vec<Value>),
}

/// A result being built.
pub struct JsonOutput<'s> {
    root_schema: &'s Value,
    /// The schema of each open frame (`None` where the schema does not know the name).
    /// The schema of each open frame; a place whose schema is not known has `Value::Null`, which declares nothing.
    schemas: Vec<&'s Value>,
    names: Vec<String>,
    stack: Vec<Frame>,
    /// Per open frame: is it sensitive (declared so, or inside a sensitive place)?
    sensitive: Vec<bool>,
    result: Option<Value>,
    problem: Option<String>,
    /// Who may see sensitive places; without it nobody does.
    access: Option<Access<'s>>,
}

/// What a result says when a sensitive place was held back. It names no place and no value.
pub const WITHHELD: &str =
    "the result holds data of a person that this connection has no release for";

impl<'s> JsonOutput<'s> {
    /// An empty result for a tool with this output schema.
    #[must_use]
    pub fn new(output_schema: &'s Value) -> Self {
        Self {
            root_schema: output_schema,
            schemas: Vec::new(),
            names: Vec::new(),
            stack: Vec::new(),
            sensitive: Vec::new(),
            result: None,
            problem: None,
            access: None,
        }
    }

    /// The same, for a connection: a sensitive place (`x-fs-sensitive` in the schema) is written only if the grants say yes at
    /// the moment it is written. Without this, no sensitive place is written, ever.
    #[must_use]
    pub const fn with_access(mut self, access: Access<'s>) -> Self {
        self.access = Some(access);
        self
    }

    /// The finished result, if the walk ended and nothing was wrong.
    ///
    /// # Errors
    /// The text says why the result cannot be shown (an unknown enumerant, a capability, an unreadable value, or no result).
    pub fn finish(self) -> Result<Value, String> {
        match (self.problem, self.result) {
            (Some(p), _) => Err(p),
            (None, Some(v)) => Ok(v),
            (None, None) => Err("the result is empty".to_owned()),
        }
    }

    /// Why the result cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    fn mark(&mut self, text: String) {
        self.problem.get_or_insert(text);
    }

    fn child_schema(&self, name: &str) -> Option<&'s Value> {
        let parent = *self.schemas.last()?;
        match self.stack.last()? {
            Frame::Object(_) => schema::property(self.root_schema, parent, name),
            Frame::List(_) => schema::items(self.root_schema, parent),
        }
    }

    /// Is `name` not declared by the contract at this place? FAIL CLOSED: where the schema of the enclosing object or list is
    /// not known, or is not an object or a list, nothing is declared. A program that gives more than its contract says is a
    /// leak waiting to happen, so the whole result is refused.
    fn undeclared(&self, name: &str) -> bool {
        let (Some(frame), Some(parent)) = (self.stack.last(), self.schemas.last()) else {
            return false; // the root of the result has no parent
        };
        let parent = schema::resolve(self.root_schema, parent);
        let kind = parent.get("type").and_then(Value::as_str);
        let (declared, wanted) = match frame {
            Frame::Object(_) => (
                parent.get("properties").and_then(|p| p.get(name)).is_some(),
                "object",
            ),
            Frame::List(_) => (parent.get("items").is_some(), "array"),
        };
        // declared by a schema of the right kind (or of no kind): anything else, also an unknown schema, declares nothing
        !(declared && (kind.is_none() || kind == Some(wanted)))
    }

    /// Does the schema of the enclosing object (or list) mark the place `name` as sensitive? Read from the property itself, before a
    /// reference is resolved: the mark sits next to the `$ref`.
    fn declared_sensitive(&self, name: &str) -> bool {
        let Some(parent) = self.schemas.last() else {
            return self
                .root_schema
                .get("x-fs-sensitive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        };
        let parent = schema::resolve(self.root_schema, parent);
        let own = match self.stack.last() {
            Some(Frame::Object(_)) => parent.get("properties").and_then(|p| p.get(name)),
            _ => parent.get("items"),
        };
        own.and_then(|s| s.get("x-fs-sensitive"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// The place of `name` in the result, as the schema writes it (`entries.text`; list elements add nothing).
    fn path_of(&self, name: &str) -> String {
        let mut parts: Vec<&str> = self.names.iter().map(String::as_str).collect();
        parts.push(name);
        parts.retain(|p| !p.is_empty());
        parts.join(".")
    }

    /// Is `name` sensitive here, and if it is: does the recipient have a release now? `true` = fine to write. A sensitive place
    /// that is held back makes the whole result an error (the text names nothing).
    fn admitted(&mut self, name: &str) -> bool {
        let inherited = self.sensitive.last().copied().unwrap_or(false);
        if !(inherited || self.declared_sensitive(name)) {
            return true;
        }
        let ok = self.access.is_some_and(|a| a.may_see(&self.path_of(name)));
        if !ok {
            self.mark(WITHHELD.to_owned());
        }
        ok
    }

    /// The name of an enumerant as the schema writes it: the one in its `enum` list whose kebab form is `kebab_name`, else
    /// the camel form.
    fn contract_name(&self, field: &str, kebab_name: &str) -> String {
        let listed = self
            .child_schema(field)
            .and_then(|s| s.get("enum")?.as_array())
            .and_then(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .find(|n| kebab(n) == kebab_name)
            });
        listed.map_or_else(|| crate::program_marks::camel(kebab_name), str::to_owned)
    }

    fn put(&mut self, name: &str, v: Value) {
        match self.stack.last_mut() {
            Some(Frame::Object(m)) => {
                m.insert(name.to_owned(), v);
            }
            Some(Frame::List(l)) => l.push(v),
            None => self.result = Some(v),
        }
    }

    fn open(&mut self, name: &str, frame: Frame) {
        if self.undeclared(name) {
            self.mark(format!("`{name}` is not a field of the contract"));
        }
        let sensitive =
            self.sensitive.last().copied().unwrap_or(false) || self.declared_sensitive(name);
        // asked at the opening of the place as well as at every field in it: a struct that is held back is held back whole
        self.admitted(name);
        let s = if self.stack.is_empty() {
            Some(schema::resolve(self.root_schema, self.root_schema))
        } else {
            self.child_schema(name)
        };
        self.schemas.push(s.unwrap_or(&UNKNOWN));
        self.names.push(name.to_owned());
        self.sensitive.push(sensitive);
        self.stack.push(frame);
    }
}

fn number(n: Option<Number>) -> Value {
    n.map_or(Value::Null, Value::Number)
}

impl Output for JsonOutput<'_> {
    fn begin_struct(&mut self, name: &str, _ty_name: &str) {
        self.open(name, Frame::Object(Map::new()));
    }

    fn begin_list(&mut self, name: &str, _ty_name: &str) {
        self.open(name, Frame::List(Vec::new()));
    }

    fn field(&mut self, name: &str, value: V<'_>) {
        if matches!(value, V::Void) {
            // a `Void` is left out: the codec writes nothing for it and the schema does not require it
            return;
        }
        if self.undeclared(name) {
            self.mark(format!("`{name}` is not a field of the contract"));
            return;
        }
        if !self.admitted(name) {
            return; // held back: the value is not put anywhere
        }
        let wide = self.child_schema(name).is_some_and(schema::is_wide_int);
        let v = match value {
            // a `Void` is left out: the codec writes nothing for a null pointer and the schema does not require it
            V::Void => return,
            V::Bool(b) => Value::Bool(b),
            V::Int(i) if wide => Value::String(i.to_string()),
            V::Uint(u) if wide => Value::String(u.to_string()),
            V::Int(i) => Value::Number(i.into()),
            V::Uint(u) => Value::Number(u.into()),
            V::Float(f) if f.is_nan() => Value::String("NaN".to_owned()),
            V::Float(f) if f == f64::INFINITY => Value::String("Infinity".to_owned()),
            V::Float(f) if f == f64::NEG_INFINITY => Value::String("-Infinity".to_owned()),
            V::Float(f) => number(Number::from_f64(f)),
            V::Text(s) => Value::String(s.to_owned()),
            V::Data(b) => Value::String(base64::encode(b)),
            // the command code names an enumerant in kebab form; the schema lists the names of the contract
            V::Enum(n) => Value::String(self.contract_name(name, n)),
            V::UnknownEnum(n) => {
                self.mark(format!(
                    "`{name}` holds the enumerant {n}, which this version does not know"
                ));
                return;
            }
            V::Capability(c) => {
                self.mark(format!(
                    "`{name}` is a capability ({c}) and cannot be shown"
                ));
                return;
            }
            V::Unreadable(why) => {
                self.mark(format!("`{name}` cannot be read: {why}"));
                return;
            }
        };
        self.put(name, v);
    }

    fn end(&mut self) {
        let frame = self.stack.pop();
        self.schemas.pop();
        self.sensitive.pop();
        let name = self.names.pop().unwrap_or_default();
        let v = match frame {
            Some(Frame::Object(m)) => Value::Object(m),
            Some(Frame::List(l)) => Value::Array(l),
            None => return,
        };
        self.put(&name, v);
    }
}
