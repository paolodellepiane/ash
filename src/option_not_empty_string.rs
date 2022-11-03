use std::{fmt::Debug, fmt::Display, ops::Deref};

pub struct OptionNotEmptyString(Option<String>);

impl OptionNotEmptyString {
    fn new(s: String) -> Self {
        if s.is_empty() {
            return OptionNotEmptyString(None);
        }
        OptionNotEmptyString(Some(s))
    }
}

impl From<String> for OptionNotEmptyString {
    fn from(s: String) -> Self {
        OptionNotEmptyString::new(s)
    }
}

impl From<&str> for OptionNotEmptyString {
    fn from(s: &str) -> Self {
        OptionNotEmptyString::new(s.to_string())
    }
}

impl From<Option<String>> for OptionNotEmptyString {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            OptionNotEmptyString::from(s)
        } else {
            OptionNotEmptyString(None)
        }
    }
}

impl From<Option<&str>> for OptionNotEmptyString {
    fn from(s: Option<&str>) -> Self {
        if let Some(s) = s {
            OptionNotEmptyString::from(s)
        } else {
            OptionNotEmptyString(None)
        }
    }
}

impl Deref for OptionNotEmptyString {
    type Target = Option<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for OptionNotEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_deref())
    }
}

impl Display for OptionNotEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = self.as_deref() {
            write!(f, "{}", s)
        } else {
            write!(f, "None")
        }
    }
}
