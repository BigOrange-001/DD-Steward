//! Generic `.darkest` text format.
//!
//! A file is a sequence of records. A record starts with a `key:` token and
//! owns every `.param value...` group up to the next `key:`. Groups may span
//! lines (see `scripts/map_generator.darkest`). `//` starts a comment that
//! runs to end of line, except inside a quoted string.
//!
//! Values are kept as raw text; typed readers decide what they mean (for
//! example whether `72.5%` and `0.725` are the same thing in a given field).

use std::path::Path;

/// One parsed file.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub records: Vec<Record>,
    /// Recoverable oddities found while parsing (stray quotes, junk tokens).
    pub warnings: Vec<Warning>,
}

/// `key: .a 1 2 .b "x"`
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// Key without the trailing colon.
    pub key: String,
    /// 1-based line of the key token.
    pub line: u32,
    pub params: Vec<Param>,
}

/// `.name v1 v2 ...` (zero or more values).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Name without the leading dot.
    pub name: String,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// Token text with surrounding quotes removed.
    pub text: String,
    pub quoted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub line: u32,
    pub message: String,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// The file is not text (some `network*` files are binary).
    NotText,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn parse_str(text: &str) -> Document {
    let mut doc = Document {
        records: vec![],
        warnings: vec![],
    };
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        }
        if c.is_whitespace() {
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    line += 1;
                    break;
                }
            }
            continue;
        }
        let start = line;
        let quoted = c == '"';
        let mut token = String::new();
        if quoted {
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\n' {
                    line += 1;
                }
                if c == '\\' && matches!(chars.peek(), Some('"' | '\\')) {
                    token.push(chars.next().unwrap());
                } else {
                    token.push(c);
                }
            }
            if !closed {
                doc.warnings.push(Warning {
                    line: start,
                    message: "unterminated string".into(),
                });
            }
        } else {
            token.push(c);
            while let Some(&next) = chars.peek() {
                // Some original values directly adjoin the next parameter,
                // e.g. 3%.required_performer_hp_range or false.required_... .
                // Decimal dots and dots in identifiers/paths remain values.
                if next == '.'
                    && (token == "true"
                        || token == "false"
                        || token
                            .strip_suffix('%')
                            .unwrap_or(&token)
                            .parse::<f64>()
                            .is_ok())
                {
                    let mut probe = chars.clone();
                    probe.next();
                    if probe
                        .peek()
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
                    {
                        break;
                    }
                }
                if next.is_whitespace() {
                    break;
                }
                if next == '/' {
                    let mut probe = chars.clone();
                    probe.next();
                    if probe.peek() == Some(&'/') {
                        break;
                    }
                }
                token.push(chars.next().unwrap());
            }
        }
        if !quoted && token == "." {
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                if chars.next() == Some('\n') {
                    line += 1;
                }
            }
            if chars
                .peek()
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
            {
                while chars
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    token.push(chars.next().unwrap());
                }
            }
        }
        let separator = token.strip_prefix('/').unwrap_or(&token);
        if !quoted
            && separator.len() >= 3
            && separator.chars().all(|c| c == '-')
            && text
                .lines()
                .nth(start.saturating_sub(1) as usize)
                .is_some_and(|l| l.trim() == token)
        {
            continue;
        }
        if !quoted && token.ends_with(':') {
            doc.records.push(Record {
                key: token[..token.len() - 1].into(),
                line: start,
                params: vec![],
            });
        } else if !quoted
            && token.starts_with('.')
            && token
                .chars()
                .nth(1)
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            if let Some(record) = doc.records.last_mut() {
                record.params.push(Param {
                    name: token[1..].into(),
                    values: vec![],
                });
            } else {
                doc.warnings.push(Warning {
                    line: start,
                    message: format!("parameter without record: {token}"),
                });
            }
        } else if let Some(param) = doc.records.last_mut().and_then(|r| r.params.last_mut()) {
            param.values.push(Value {
                text: token,
                quoted,
            });
        } else {
            doc.warnings.push(Warning {
                line: start,
                message: format!("value without parameter: {token}"),
            });
        }
    }
    doc
}

pub fn parse_file(path: &Path) -> Result<Document, Error> {
    let bytes = crate::telemetry024c::read(path)?;
    if bytes.contains(&0) {
        return Err(Error::NotText);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::NotText)?;
    Ok(parse_str(text))
}

impl Record {
    /// All params with this name, in file order (names can repeat).
    pub fn params_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Param> + 'a {
        self.params.iter().filter(move |p| p.name == name)
    }

    /// First param with this name.
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }
}

impl Document {
    pub fn records_keyed<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a Record> + 'a {
        self.records.iter().filter(move |r| r.key == key)
    }
}

