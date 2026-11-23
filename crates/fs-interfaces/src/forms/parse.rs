//! Reading a catalog text: strict, a bad line is an error with its number, never skipped. Pattern: Parser (one line, one rule).

use std::fmt;

use super::types::{Art, Channel, Count, Form, Status, Version, number};

/// Why a line was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Not of the form `name@H.N | display | status | description` or `> channel | art | required | count`.
    Shape,
    /// The form name is not lower-case words joined by `-`.
    Name,
    /// The version is not `H.N`.
    Version,
    /// The status is neither `stable` nor `planned`.
    Status,
    /// The same `name@H.N` twice.
    DuplicateForm,
    /// A channel line before any form line.
    ChannelWithoutForm,
    /// The channel name is not lower-case words joined by `-`.
    ChannelName,
    /// The same channel twice in one form.
    DuplicateChannel,
    /// The art is not one of the seven words (or the set is empty or has a blank element).
    Art,
    /// The same art twice in one set.
    ArtDuplicate,
    /// The required mark is neither `!` nor `-`.
    Required,
    /// The count is not `1`, `a..b`, `a..n` or `?`.
    Count,
    /// A required channel whose count allows none (`0..`).
    RequiredButOptionalCount,
    /// A form without any channel.
    NoChannel,
    /// A form without a required channel (a module could fill nothing).
    NoRequiredChannel,
}

/// A refused line (number counted from 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The line number.
    pub line: usize,
    /// Why.
    pub kind: ErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "forms line {}: {:?}", self.line, self.kind)
    }
}

impl std::error::Error for ParseError {}

fn is_word_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn parse_count(text: &str) -> Option<Count> {
    if text == "?" {
        return Some(Count::Open);
    }
    if let Some(n) = number(text) {
        return Some(Count::Range {
            min: n,
            max: Some(n),
        });
    }
    let (min, max) = text.split_once("..")?;
    let min = number(min)?;
    let max = if max == "n" { None } else { Some(number(max)?) };
    if max.is_some_and(|m| m < min) {
        return None;
    }
    Some(Count::Range { min, max })
}

fn parse_form(fields: &[&str]) -> Result<Form, ErrorKind> {
    let [key, display_name, status, description] = fields else {
        return Err(ErrorKind::Shape);
    };
    let (name, version) = key.split_once('@').ok_or(ErrorKind::Shape)?;
    if !is_word_name(name) {
        return Err(ErrorKind::Name);
    }
    let version = Version::parse(version).ok_or(ErrorKind::Version)?;
    let status = match *status {
        "stable" => Status::Stable,
        "planned" => Status::Planned,
        _ => return Err(ErrorKind::Status),
    };
    if display_name.is_empty() || description.is_empty() {
        return Err(ErrorKind::Shape);
    }
    Ok(Form {
        name: (*name).to_owned(),
        version,
        display_name: (*display_name).to_owned(),
        status,
        description: (*description).to_owned(),
        channels: Vec::new(),
    })
}

/// `text` or a set `text|number|time` (no spaces, only the seven arts, no duplicate, not empty).
fn parse_arts(text: &str) -> Result<Vec<Art>, ErrorKind> {
    let mut arts: Vec<Art> = Vec::new();
    for word in text.split('|') {
        let art = Art::parse(word).ok_or(ErrorKind::Art)?;
        if arts.contains(&art) {
            return Err(ErrorKind::ArtDuplicate);
        }
        arts.push(art);
    }
    Ok(arts)
}

fn parse_channel(fields: &[&str]) -> Result<Channel, ErrorKind> {
    let [name, art, required, count] = fields else {
        return Err(ErrorKind::Shape);
    };
    if !is_word_name(name) {
        return Err(ErrorKind::ChannelName);
    }
    let arts = parse_arts(art)?;
    let required = match *required {
        "!" => true,
        "-" => false,
        _ => return Err(ErrorKind::Required),
    };
    let count = parse_count(count).ok_or(ErrorKind::Count)?;
    if required && matches!(count, Count::Range { min: 0, .. }) {
        return Err(ErrorKind::RequiredButOptionalCount);
    }
    Ok(Channel {
        name: (*name).to_owned(),
        arts,
        required,
        count,
    })
}

/// Reads a catalog text. Blank lines and lines starting with `#` are skipped; any other line must be a form line or a channel line.
///
/// # Errors
/// The first bad line, with its number: wrong shape, bad name, version, status, art, mark or count, a duplicate, a channel before a form,
/// a form without channels or without a required channel.
pub fn parse(text: &str) -> Result<Vec<Form>, ParseError> {
    let mut forms: Vec<Form> = Vec::new();
    let mut form_line = 0;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let err = |kind| ParseError { line, kind };
        if let Some(rest) = trimmed.strip_prefix('>') {
            let fields: Vec<&str> = rest.trim().split(" | ").map(str::trim).collect();
            let channel = parse_channel(&fields).map_err(err)?;
            let form = forms
                .last_mut()
                .ok_or_else(|| err(ErrorKind::ChannelWithoutForm))?;
            if form.channel(&channel.name).is_some() {
                return Err(err(ErrorKind::DuplicateChannel));
            }
            form.channels.push(channel);
        } else {
            check_complete(forms.last(), form_line)?;
            let fields: Vec<&str> = trimmed.splitn(4, '|').map(str::trim).collect();
            let form = parse_form(&fields).map_err(err)?;
            if forms
                .iter()
                .any(|f| f.name == form.name && f.version == form.version)
            {
                return Err(err(ErrorKind::DuplicateForm));
            }
            form_line = line;
            forms.push(form);
        }
    }
    check_complete(forms.last(), form_line)?;
    Ok(forms)
}

fn check_complete(form: Option<&Form>, line: usize) -> Result<(), ParseError> {
    let Some(form) = form else { return Ok(()) };
    if form.channels.is_empty() {
        return Err(ParseError {
            line,
            kind: ErrorKind::NoChannel,
        });
    }
    if form.required().next().is_none() {
        return Err(ParseError {
            line,
            kind: ErrorKind::NoRequiredChannel,
        });
    }
    Ok(())
}
