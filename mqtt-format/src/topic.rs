use thiserror::Error;

/// The topic level seperator
///
/// Defined in 4.7.1.1
pub const TOPIC_LEVEL_SEPERATOR: char = '/';

/// The maximum length of a topic name or filter
///
/// Defined in 4.7.3
pub const MAXIMUM_TOPIC_BYTE_LENGTH: usize = 65535;

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct TopicPath(String);

impl PartialEq<str> for TopicPath {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl TryFrom<String> for TopicPath {
    type Error = TopicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains(['+', '#']) {
            Err(TopicError::MixedWildcardLevel)
        } else if value.contains('\0') {
            Err(TopicError::NullCharacter)
        } else {
            Ok(TopicPath(value))
        }
    }
}

impl core::ops::Deref for TopicPath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum TopicFilterLevel {
    Path(TopicPath),
    TopicLevelSeperator,
    MultiLevelSeperator,
    Empty,
}

impl TopicFilterLevel {
    /// Returns `true` if the topic level is [`MultiLevelSeperator`].
    ///
    /// [`MultiLevelSeperator`]: TopicLevel::MultiLevelSeperator
    #[must_use]
    pub fn is_multi_level_seperator(&self) -> bool {
        matches!(self, Self::MultiLevelSeperator)
    }

    /// Returns `true` if the topic level is [`TopicLevelSeperator`].
    ///
    /// [`TopicLevelSeperator`]: TopicLevel::TopicLevelSeperator
    #[must_use]
    pub fn is_topic_level_seperator(&self) -> bool {
        matches!(self, Self::TopicLevelSeperator)
    }

    /// Returns `true` if the topic level is [`Path`].
    ///
    /// [`Path`]: TopicLevel::Path
    #[must_use]
    pub fn is_path(&self) -> bool {
        matches!(self, Self::Path(..))
    }

    /// Return a str representation of this topic level
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            TopicFilterLevel::Path(p) => p,
            TopicFilterLevel::TopicLevelSeperator => "+",
            TopicFilterLevel::MultiLevelSeperator => "#",
            TopicFilterLevel::Empty => "",
        }
    }
}

impl TryFrom<String> for TopicFilterLevel {
    type Error = TopicError;
    fn try_from(level: String) -> Result<Self, Self::Error> {
        if level.contains(TOPIC_LEVEL_SEPERATOR) {
            return Err(TopicError::LevelContainsSeperator);
        }

        Ok(match level.as_str() {
            "" => TopicFilterLevel::Empty,
            "+" => TopicFilterLevel::TopicLevelSeperator,
            "#" => TopicFilterLevel::MultiLevelSeperator,
            _ => TopicFilterLevel::Path(level.try_into()?),
        })
    }
}

#[derive(Debug, Error)]
pub enum TopicError {
    #[error("An individual topic level contained a separator")]
    /// An individual topic level contained a seperator
    LevelContainsSeperator,
    #[error("Cannot append to a topic name or filter with a multi level wildcard")]
    CannotAppendToMultilevelWildcard,
    #[error("A topic name or filter cannot be empty")]
    Empty,
    #[error("Topic filters cannot contain a wildcard and other characters on the same level")]
    MixedWildcardLevel,
    #[error("Appending the topic to this topic name or filter would exceed the maximum length")]
    WouldExceedLength,
    #[error("Topic names or filters cannot contain a null character")]
    NullCharacter,
}

/// An owned MQTT Topic Filter
///
/// A topic filter is denoted as a string like `"sport/tennis/player1/#"`. They are commonly used
/// around subscriptions. For publishing, a [`TopicNameBuf`] is used.
///
/// As defined in 4.7.3
#[derive(Debug, Clone, Hash, PartialEq)]
pub struct TopicFilterBuf {
    levels: Vec<TopicFilterLevel>,
    has_end_wildcard: bool,
}

impl TopicFilterBuf {
    pub fn new(levels: impl AsRef<str>) -> Result<TopicFilterBuf, TopicError> {
        if levels.as_ref().is_empty() {
            return Err(TopicError::Empty);
        }
        let mut topic = TopicFilterBuf {
            levels: vec![],
            has_end_wildcard: false,
        };
        topic.append(levels)?;
        Ok(topic)
    }

    pub fn append_single(
        &mut self,
        level: impl TryInto<TopicFilterLevel, Error = TopicError>,
    ) -> Result<(), TopicError> {
        if self.has_end_wildcard {
            return Err(TopicError::CannotAppendToMultilevelWildcard);
        }
        let level = level.try_into()?;

        if self.len() + 1 + level.as_str().len() > MAXIMUM_TOPIC_BYTE_LENGTH {
            return Err(TopicError::WouldExceedLength);
        }

        if level.is_multi_level_seperator() {
            self.has_end_wildcard = true;
        }

        self.levels.push(level);

        Ok(())
    }

    pub fn append(&mut self, levels: impl AsRef<str>) -> Result<(), TopicError> {
        levels
            .as_ref()
            .split(TOPIC_LEVEL_SEPERATOR)
            .try_for_each(|next| self.append_single(next.to_string()))
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.levels.len() + self.levels.iter().map(|l| l.as_str().len()).sum::<usize>()
    }

    pub fn first(&self) -> &TopicFilterLevel {
        self.levels.first().unwrap()
    }

    pub fn last(&self) -> &TopicFilterLevel {
        self.levels.last().unwrap()
    }
}

/// An owned MQTT Topic Name
///
/// A topic name is denoted as a string like `"sport/tennis/player1/score"`. They are commonly used
/// around publishing. For subscriptions, a [`TopicFilterBuf`] is used.
///
/// As defined in 4.7.3
#[derive(Debug, Clone, Hash, PartialEq)]
pub struct TopicNameBuf {
    levels: Vec<TopicPath>,
}

impl TopicNameBuf {
    pub fn new(levels: impl AsRef<str>) -> Result<TopicNameBuf, TopicError> {
        if levels.as_ref().is_empty() {
            return Err(TopicError::Empty);
        }
        let mut topic = TopicNameBuf { levels: vec![] };
        topic.append(levels)?;
        Ok(topic)
    }

    pub fn append_single(
        &mut self,
        level: impl TryInto<TopicPath, Error = TopicError>,
    ) -> Result<(), TopicError> {
        let level = level.try_into()?;

        if self.len() + 1 + level.as_str().len() > MAXIMUM_TOPIC_BYTE_LENGTH {
            return Err(TopicError::WouldExceedLength);
        }

        self.levels.push(level);

        Ok(())
    }

    pub fn append(&mut self, levels: impl AsRef<str>) -> Result<(), TopicError> {
        levels
            .as_ref()
            .split(TOPIC_LEVEL_SEPERATOR)
            .try_for_each(|next| self.append_single(next.to_string()))
    }

    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.levels.len() + self.levels.iter().map(|l| l.as_str().len()).sum::<usize>()
    }

    pub fn matches(&self, filter: &TopicFilterBuf) -> bool {
        if filter.first().is_multi_level_seperator() {
            return true;
        }

        if filter.has_end_wildcard && self.levels.len() < filter.levels.len() {
            return false;
        }

        for (filter, path) in filter.levels.iter().zip(&self.levels) {
            match filter {
                TopicFilterLevel::Path(topic_path) => {
                    if topic_path != path {
                        return false;
                    }
                }
                TopicFilterLevel::Empty => {
                    if path != "" {
                        return false;
                    }
                }
                TopicFilterLevel::TopicLevelSeperator => {}
                TopicFilterLevel::MultiLevelSeperator => return true,
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_topic_names() {
        let topics = ["sport/tennis", "sport/foo", "/", "/asdf", "asda/"];

        for topic in topics {
            TopicNameBuf::new(topic).unwrap();
        }
    }

    #[test]
    fn invalid_topic_names() {
        let topics = ["sport/tennis+", "sport/foo#", "", "/aa#", "#/asd", "foo/#/"];

        for topic in topics {
            TopicNameBuf::new(topic).unwrap_err();
        }
    }

    #[test]
    fn check_matching() {
        let matches = [
            ("foo/bar", "foo/bar"),
            ("foo/bar", "foo/+"),
            ("foo/bar", "+/+"),
            ("foo/bar", "foo/#"),
            ("foo/bar", "#"),
            ("foo/bar", "foo/bar/#"),
        ];

        for (name, filter) in matches {
            let name = TopicNameBuf::new(name).unwrap();
            let filter = TopicFilterBuf::new(filter).unwrap();

            assert!(
                name.matches(&filter),
                "{name:?} and {filter:?} did not match"
            );
        }
    }
}
