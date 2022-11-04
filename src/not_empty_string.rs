use std::{fmt::Debug, fmt::Display, ops::Deref};

pub struct NotEmptyString(Option<String>);

impl NotEmptyString {
    fn new(s: String) -> Self {
        if s.is_empty() {
            return NotEmptyString(Option::None);
        }
        NotEmptyString(Some(s))
    }

    pub fn none() -> Self {
        NotEmptyString(Option::None)
    }
}

impl From<String> for NotEmptyString {
    fn from(s: String) -> Self {
        NotEmptyString::new(s)
    }
}

impl From<&str> for NotEmptyString {
    fn from(s: &str) -> Self {
        NotEmptyString::new(s.to_string())
    }
}

impl From<Option<String>> for NotEmptyString {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            NotEmptyString::from(s)
        } else {
            NotEmptyString(Option::None)
        }
    }
}

impl From<Option<&str>> for NotEmptyString {
    fn from(s: Option<&str>) -> Self {
        if let Some(s) = s {
            NotEmptyString::from(s)
        } else {
            NotEmptyString(Option::None)
        }
    }
}

impl Deref for NotEmptyString {
    type Target = Option<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for NotEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_deref())
    }
}

impl Display for NotEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = self.as_deref() {
            write!(f, "{}", s)
        } else {
            write!(f, "None")
        }
    }
}
