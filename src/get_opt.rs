use std::{borrow::Cow, collections::VecDeque, iter::Peekable};

#[derive(PartialEq)]
pub enum ArgType {
    None,
    Required,
    Optional,
}

pub struct LongOption<'a> {
    pub name: &'a str,
    pub arg: ArgType,
    pub flag: Option<char>,
}

impl<'a> From<(&'a str, ArgType, Option<char>)> for LongOption<'a> {
    fn from((name, arg, flag): (&'a str, ArgType, Option<char>)) -> Self {
        Self { name, arg, flag }
    }
}

pub struct GetOpt<'a, T: Iterator<Item = String>> {
    args: Peekable<T>,
    chars: VecDeque<char>,
    rest: Vec<String>,

    short_opts: &'a str,
    long_opts: &'a [LongOption<'a>],

    all_args: bool,
    posixly_correct: bool,
    long_only: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GetOptError {
    ShortOptionInvalid(char),
    ShortOptionMissingArgument(char),
    LongOptionMissingArgument(String),
    LongOptionUnexpectedArgument(String),
    AmbiguousOption(String),
    UnrecognizedOption(String),
}

impl std::fmt::Display for GetOptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShortOptionInvalid(opt) => write!(f, "invalid option -- {opt}"),
            Self::ShortOptionMissingArgument(opt) => {
                write!(f, "option requires an argument -- {opt}")
            }
            Self::LongOptionMissingArgument(opt) => {
                write!(f, "option `{opt}' requires an argument")
            }
            Self::AmbiguousOption(opt) => write!(f, "option `{opt}' is ambiguous"),
            Self::UnrecognizedOption(opt) => write!(f, "unrecognized option `{opt}'"),
            Self::LongOptionUnexpectedArgument(opt) => {
                write!(f, "option `{opt}' doesn't allow an argument")
            }
        }
    }
}

impl std::error::Error for GetOptError {}

#[derive(Debug)]
pub enum Arg<'a> {
    Flag(char, Option<String>),
    Name(&'a str, Option<String>),
    Positional(String),
}

#[derive(Debug)]
pub enum ArgRef<'a> {
    Flag(char, Option<&'a str>),
    Name(&'a str, Option<&'a str>),
    Positional(&'a str),
}

impl<'a> Arg<'a> {
    pub fn as_ref(&self) -> ArgRef {
        match self {
            Arg::Flag(c, v) => ArgRef::Flag(*c, v.as_deref()),
            Arg::Name(n, v) => ArgRef::Name(n, v.as_deref()),
            Arg::Positional(p) => ArgRef::Positional(p.as_str()),
        }
    }
}

impl<'a, T: Iterator<Item = String>> GetOpt<'a, T> {
    pub fn new(args: T, short_opts: &'a str, long_opts: &'a [LongOption], long_only: bool) -> Self {
        let mut args = args.peekable();
        let program_name = args.next();
        Self {
            args,
            chars: VecDeque::new(),
            all_args: short_opts.starts_with('-'),
            posixly_correct: short_opts.starts_with('+'),
            long_only,
            short_opts: short_opts.strip_prefix(['+', '-']).unwrap_or(short_opts),
            long_opts,
            rest: program_name.into_iter().collect(),
        }
    }

    pub fn rest(self) -> impl Iterator<Item = String> {
        self.rest.into_iter()
    }

    fn parse_short_opts(&mut self) -> Result<Arg<'a>, GetOptError> {
        let opt = self
            .chars
            .pop_front()
            .expect("shouldn't have been called when not processing short options");

        let arg_type = self
            .short_opts
            .find(opt)
            .filter(|_| opt != ':')
            .map(
                |i| match self.short_opts.as_bytes().get(i + opt.len_utf8()..) {
                    Some(&[b':', b':', ..]) => ArgType::Optional,
                    Some(&[b':', ..]) => ArgType::Required,
                    _ => ArgType::None,
                },
            )
            .ok_or_else(|| GetOptError::ShortOptionInvalid(opt))?;

        let value = match arg_type {
            ArgType::None => None,
            ArgType::Optional => (!self.chars.is_empty()).then(|| self.chars.drain(..).collect()),
            ArgType::Required => (!self.chars.is_empty())
                .then(|| self.chars.drain(..).collect::<String>())
                .or_else(|| self.args.next())
                .map(Some)
                .ok_or(GetOptError::ShortOptionMissingArgument(opt))?,
        };

        Ok(Arg::Flag(opt, value))
    }

    fn parse_long_opts(&mut self, prefix: &str, opt: &str) -> Result<Arg<'a>, GetOptError> {
        let option = format!("{prefix}{opt}");

        let opt: Cow<str> = match (prefix, opt) {
            ("-W", "") => self
                .args
                .next()
                .map(Cow::Owned)
                .ok_or_else(|| GetOptError::LongOptionMissingArgument(prefix.into()))?,
            _ => Cow::Borrowed(opt),
        };

        let (key, value) = opt
            .split_once('=')
            .map(|(k, v)| (k, Some(v.to_string())))
            .unwrap_or((opt.as_ref(), None));

        let long_opt = match self.long_opts.iter().find(|lo| lo.name == key) {
            Some(opt) => opt,
            None => {
                if let Ok(c) = opt.parse::<char>() {
                    self.chars.push_back(c);
                    return self.parse_short_opts();
                }

                let mut matches = self.long_opts.iter().filter(|lo| lo.name.starts_with(key));
                if let Some(first) = matches.next() {
                    if matches.any(|m| self.long_only || first.arg != m.arg || first.flag != m.flag)
                    {
                        return Err(GetOptError::AmbiguousOption(option));
                    }

                    first
                } else {
                    if key.starts_with(|c| self.short_opts.contains(c)) {
                        self.chars.extend(key.chars());
                        return self.parse_short_opts();
                    } else {
                        return Err(GetOptError::UnrecognizedOption(option));
                    }
                }
            }
        };

        let value = match long_opt.arg {
            ArgType::None if value.is_some() => {
                Err(GetOptError::LongOptionUnexpectedArgument(option))?
            }
            ArgType::Required => value
                .or_else(|| self.args.next())
                .map(Some)
                .ok_or(GetOptError::LongOptionMissingArgument(option))?,
            _ => value,
        };

        Ok(match long_opt.flag {
            Some(f) => Arg::Flag(f, value),
            None => Arg::Name(long_opt.name, value),
        })
    }

    pub fn iter(&mut self) -> &mut Self {
        self
    }
}

impl<'a, T: Iterator<Item = String>> Iterator for GetOpt<'a, T> {
    type Item = Result<Arg<'a>, GetOptError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.chars.is_empty() {
            return Some(self.parse_short_opts());
        }

        let opt = self.args.next()?;
        if !opt.starts_with("-") || opt == "-" {
            return if self.posixly_correct {
                self.rest.push(opt);
                self.rest.extend(&mut self.args);
                None
            } else if self.all_args {
                Some(Ok(Arg::Positional(opt)))
            } else {
                self.rest.push(opt);
                self.next()
            };
        }

        if opt == "--" {
            self.rest.extend(self.args.by_ref());
            return None;
        }

        if let Some((prefix, opt)) = [
            ("--", true),
            ("-", self.long_only),
            ("-W", self.short_opts.contains("W;")),
        ]
        .iter()
        .find_map(|&(p, pred)| pred.then(|| opt.strip_prefix(p))?.map(|opt| (p, opt)))
        {
            return Some(self.parse_long_opts(prefix, opt));
        }

        self.chars.extend(opt.trim_start_matches("-").chars());
        self.next()
    }
}
