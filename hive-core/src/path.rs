pub use regex::Error as RegexError;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

static PATH_PARAMS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r":([^/]+)|\*").unwrap());

pub type Params = HashMap<Box<str>, Box<str>>;

#[derive(Debug)]
pub struct PathMatcher {
  path: Box<str>,
  regex: Regex,
  param_names: Vec<Box<str>>,
}

impl PathMatcher {
  pub fn new(matcher: &str) -> Result<Self, RegexError> {
    let mut regex = "^".to_owned();
    let mut param_names = Vec::new();

    if !matcher.starts_with('/') {
      regex += "/";
    }

    let mut start_pos = 0;
    for captures in PATH_PARAMS_REGEX.captures_iter(matcher) {
      let whole = captures.get(0).unwrap();
      regex += &regex::escape(&matcher[start_pos..whole.start()]);
      if whole.as_str() == "*" {
        regex += r"(.*)";
        param_names.push("*".into())
      } else {
        regex += r"([^/]+)";
        param_names.push(captures[1].into());
      }
      start_pos = whole.end();
    }
    regex += &regex::escape(&matcher[start_pos..]);
    regex += "$";

    Ok(Self {
      path: matcher.into(),
      regex: Regex::new(&regex)?,
      param_names,
    })
  }

  pub fn gen_params(&self, path: &str) -> Option<Params> {
    self.regex.captures(path).map(|captures| {
      self
        .param_names
        .iter()
        .zip(captures.iter().skip(1))
        .filter_map(|(n, m)| m.map(|m| (n.clone(), m.as_str().into())))
        .collect()
    })
  }

  pub fn as_str(&self) -> &str {
    &self.path
  }

  pub fn as_regex_str(&self) -> &str {
    self.regex.as_str()
  }
}

impl Serialize for PathMatcher {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut x = serializer.serialize_struct("PathMatcher", 2)?;
    x.serialize_field("pattern", self.as_str())?;
    x.serialize_field("regex", self.as_regex_str())?;
    x.end()
  }
}

/// Taken from Cargo
/// <https://github.com/rust-lang/cargo/blob/af307a38c20a753ec60f0ad18be5abed3db3c9ac/src/cargo/util/paths.rs#L60-L85>
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
  let mut components = path.as_ref().components().peekable();
  let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
    components.next();
    PathBuf::from(c.as_os_str())
  } else {
    PathBuf::new()
  };

  for component in components {
    match component {
      Component::Prefix(..) => unreachable!(),
      Component::RootDir => {
        ret.push(component.as_os_str());
      }
      Component::CurDir => {}
      Component::ParentDir => {
        ret.pop();
      }
      Component::Normal(c) => {
        ret.push(c);
      }
    }
  }
  ret
}

/// Similar to `hive_core::path::normalize_path`, but for `str`s instead of
/// `Path`s.
pub fn normalize_path_str(path: &str) -> String {
  let mut result = Vec::new();
  let segments = path.split(['/', '\\']).filter(|&x| x != "" && x != ".");
  for s in segments {
    if s == ".." {
      result.pop();
    } else {
      result.push(s);
    }
  }
  result.join("/")
}
