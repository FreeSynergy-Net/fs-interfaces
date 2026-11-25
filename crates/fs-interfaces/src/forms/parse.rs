//! Reading a catalog text: strict, a bad line is an error with its number, never skipped. Pattern: Parser (one line, one rule).

use super::error::{ErrorKind, ParseError};
use super::relation::{check_relations, parse_relation};
use super::types::{Art, Channel, Count, Form, Status, Write, number, parse_version};

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
    if let Some(n) = number(text) {
        return Some(Count {
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
    Some(Count { min, max })
}

fn parse_form(fields: &[&str]) -> Result<Form, ErrorKind> {
    let [key, display_name, status, description] = fields else {
        return Err(ErrorKind::Shape);
    };
    let (name, version) = key.split_once('@').ok_or(ErrorKind::Shape)?;
    if !is_word_name(name) {
        return Err(ErrorKind::Name);
    }
    let version = parse_version(version).ok_or(ErrorKind::Version)?;
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
        relations: Vec::new(),
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

/// `column! order lane`: channel names separated by blanks, `!` behind a name = the gesture must deliver it. Empty text = writes nothing.
fn parse_writes(text: &str) -> Result<Vec<Write>, ErrorKind> {
    let mut writes: Vec<Write> = Vec::new();
    for word in text.split_whitespace() {
        let (channel, required) = word
            .strip_suffix('!')
            .map_or((word, false), |name| (name, true));
        if !is_word_name(channel) {
            return Err(ErrorKind::WritesName);
        }
        if writes.iter().any(|w| w.channel == channel) {
            return Err(ErrorKind::WritesDuplicate);
        }
        writes.push(Write {
            channel: channel.to_owned(),
            required,
        });
    }
    Ok(writes)
}

fn parse_channel(fields: &[&str]) -> Result<Channel, ErrorKind> {
    let (name, art, required, count, writes) = match fields {
        [name, art, required, count] => (name, art, required, count, None),
        [name, art, required, count, writes] => (name, art, required, count, Some(writes)),
        _ => return Err(ErrorKind::Shape),
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
    if required && count.min == 0 {
        return Err(ErrorKind::RequiredButOptionalCount);
    }
    let writes = match writes {
        None => None,
        Some(_) if arts != [Art::Command] => return Err(ErrorKind::WritesOnNonCommand),
        Some(text) => Some(parse_writes(text)?),
    };
    Ok(Channel {
        name: (*name).to_owned(),
        arts,
        required,
        count,
        writes,
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
    let mut channel_lines: Vec<usize> = Vec::new();
    let mut relation_lines: Vec<usize> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let err = |kind| ParseError { line, kind };
        if let Some(rest) = trimmed.strip_prefix('>') {
            let rest = rest.trim();
            // A trailing ` |` is an empty 5th field: a command that writes nothing.
            let (rest, empty_writes) = match rest.strip_suffix('|') {
                Some(body) if body.ends_with(' ') => (body.trim_end(), true),
                _ => (rest, false),
            };
            let mut fields: Vec<&str> = rest.split(" | ").map(str::trim).collect();
            if empty_writes {
                fields.push("");
            }
            let channel = parse_channel(&fields).map_err(err)?;
            let form = forms
                .last_mut()
                .ok_or_else(|| err(ErrorKind::ChannelWithoutForm))?;
            if form.channel(&channel.name).is_some() {
                return Err(err(ErrorKind::DuplicateChannel));
            }
            form.channels.push(channel);
            channel_lines.push(line);
        } else if let Some(rest) = trimmed.strip_prefix('=') {
            let relation = parse_relation(rest).map_err(err)?;
            let form = forms
                .last_mut()
                .ok_or_else(|| err(ErrorKind::RelationWithoutForm))?;
            form.relations.push(relation);
            relation_lines.push(line);
        } else {
            check_complete(forms.last(), form_line, &channel_lines, &relation_lines)?;
            channel_lines.clear();
            relation_lines.clear();
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
    check_complete(forms.last(), form_line, &channel_lines, &relation_lines)?;
    Ok(forms)
}

/// `writes` may name only data channels of the same form with room for one field; the error carries the line of the command channel.
fn check_writes(form: &Form, channel_lines: &[usize]) -> Result<(), ParseError> {
    for (channel, &line) in form.channels.iter().zip(channel_lines) {
        for write in channel.writes.iter().flatten() {
            let kind = match form.channel(&write.channel) {
                None => Some(ErrorKind::WritesUnknownChannel),
                Some(c) if c.arts == [Art::Command] => Some(ErrorKind::WritesCommandChannel),
                Some(c) if c.count.max != Some(1) => Some(ErrorKind::WritesManyChannel),
                Some(_) => None,
            };
            if let Some(kind) = kind {
                return Err(ParseError { line, kind });
            }
        }
    }
    Ok(())
}

fn check_complete(
    form: Option<&Form>,
    line: usize,
    channel_lines: &[usize],
    relation_lines: &[usize],
) -> Result<(), ParseError> {
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
    check_writes(form, channel_lines)?;
    match check_relations(form) {
        Some((i, kind)) => Err(ParseError {
            line: relation_lines[i],
            kind,
        }),
        None => Ok(()),
    }
}
