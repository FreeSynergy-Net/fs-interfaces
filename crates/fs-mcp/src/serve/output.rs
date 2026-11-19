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

use super::{base64, schema};

enum Frame {
    Object(Map<String, Value>),
    List(Vec<Value>),
}

/// A result being built.
pub struct JsonOutput<'s> {
    root_schema: &'s Value,
    /// The schema of each open frame (`None` where the schema does not know the name).
    schemas: Vec<Option<&'s Value>>,
    names: Vec<String>,
    stack: Vec<Frame>,
    result: Option<Value>,
    problem: Option<String>,
}

impl<'s> JsonOutput<'s> {
    /// An empty result for a tool with this output schema.
    #[must_use]
    pub fn new(output_schema: &'s Value) -> Self {
        Self {
            root_schema: output_schema,
            schemas: Vec::new(),
            names: Vec::new(),
            stack: Vec::new(),
            result: None,
            problem: None,
        }
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
        let parent = self.schemas.last().copied().flatten()?;
        match self.stack.last()? {
            Frame::Object(_) => schema::property(self.root_schema, parent, name),
            Frame::List(_) => schema::items(self.root_schema, parent),
        }
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
        let s = if self.stack.is_empty() {
            Some(schema::resolve(self.root_schema, self.root_schema))
        } else {
            self.child_schema(name)
        };
        self.schemas.push(s);
        self.names.push(name.to_owned());
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
        let name = self.names.pop().unwrap_or_default();
        let v = match frame {
            Some(Frame::Object(m)) => Value::Object(m),
            Some(Frame::List(l)) => Value::Array(l),
            None => return,
        };
        self.put(&name, v);
    }
}
