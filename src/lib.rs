//! `duration-string` is a library to convert from `String` to `Duration` and vice-versa.
//!
//! ![Crates.io](https://img.shields.io/crates/v/duration-string.svg)
//! ![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)
//!
//! Takes a `String` such as `100ms`, `2s`, `5m 30s`, `1h10m` and converts it into a `Duration`.
//!
//! Takes a `Duration` and converts it into `String`.
//!
//! The `String` format is a multiply of `[0-9]+(ns|us|ms|[smhdwy])`
//!
//! ## Example
//!
//! `String` to `Duration`:
//!
//! ```rust
//! use std::convert::TryFrom;
//! use duration_string::DurationString;
//! use std::time::Duration;
//!
//! let d: Duration = DurationString::try_from(String::from("100ms")).unwrap().into();
//! assert_eq!(d, Duration::from_millis(100));
//!
//! // Alternatively
//! let d: Duration = "100ms".parse::<DurationString>().unwrap().into();
//! assert_eq!(d, Duration::from_millis(100));
//! ```
//!
//! `Duration` to `String`:
//!
//! ```rust
//! use std::convert::TryFrom;
//! use duration_string::*;
//! use std::time::Duration;
//!
//! let d: String = DurationString::from(Duration::from_millis(100)).into();
//! assert_eq!(d, String::from("100ms"));
//! ```
//!
//! ## Serde support
//!
//! You can enable _serialization/deserialization_ support by adding the feature `serde`
//!
//! - Add `serde` feature
//!
//!    ```toml
//!    duration-string = { version = "0.3.0", features = ["serde"] }
//!    ```
//!
//! - Add derive to struct
//!
//!    ```ignore
//!    use duration_string::DurationString;
//!    use serde::{Deserialize, Serialize};
//!
//!    #[derive(Serialize, Deserialize)]
//!    struct Foo {
//!      duration: DurationString
//!    }
//!    ```
//!
#![cfg_attr(feature = "serde", doc = "```rust")]
#![cfg_attr(not(feature = "serde"), doc = "```ignore")]
//! ```
//! use duration_string::DurationString;
//! use serde::{Deserialize, Serialize};
//! use serde_json;
//!
//! #[derive(Serialize, Deserialize)]
//! struct SerdeSupport {
//!     t: DurationString,
//! }
//! let s = SerdeSupport {
//!     t: DurationString::from_string(String::from("1m")).unwrap(),
//! };
//! assert_eq!(r#"{"t":"1m"}"#, serde_json::to_string(&s).unwrap());
//! ```

#[cfg(feature = "serde")]
use serde::de::Unexpected;
use std::borrow::{Borrow, BorrowMut};
use std::convert::TryFrom;
#[cfg(feature = "serde")]
use std::fmt;
#[cfg(feature = "serde")]
use std::marker::PhantomData;
use std::num::ParseIntError;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::Duration;

const YEAR_IN_NANO: u128 = 31_556_926_000_000_000;
const WEEK_IN_NANO: u128 = 604_800_000_000_000;
const DAY_IN_NANO: u128 = 86_400_000_000_000;
const HOUR_IN_NANO: u128 = 3_600_000_000_000;
const MINUTE_IN_NANO: u128 = 60_000_000_000;
const SECOND_IN_NANO: u128 = 1_000_000_000;
const MILLISECOND_IN_NANO: u128 = 1_000_000;
const MICROSECOND_IN_NANO: u128 = 1000;

const HOUR_IN_SECONDS: u32 = 3600;
const MINUTE_IN_SECONDS: u32 = 60;
const DAY_IN_SECONDS: u32 = 86_400;
const WEEK_IN_SECONDS: u32 = 604_800;
const YEAR_IN_SECONDS: u32 = 31_556_926;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Format,
    Overflow,
    ParseInt(ParseIntError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format => write!(
                f,
                "missing time duration format, must be multiples of `[0-9]+(ns|us|ms|[smhdwy])`"
            ),
            Self::Overflow => write!(f, "number is too large to fit in target type"),
            Self::ParseInt(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format | Self::Overflow => None,
            Self::ParseInt(err) => Some(err),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct DurationString(Duration);

impl DurationString {
    #[must_use]
    pub const fn new(duration: Duration) -> DurationString {
        DurationString(duration)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn from_string(duration: String) -> Result<Self> {
        DurationString::try_from(duration)
    }
}

impl std::fmt::Display for DurationString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{s}")
    }
}

impl From<DurationString> for Duration {
    fn from(value: DurationString) -> Self {
        value.0
    }
}

impl From<DurationString> for String {
    fn from(value: DurationString) -> Self {
        let ns = value.0.as_nanos();
        if ns % YEAR_IN_NANO == 0 {
            return (ns / YEAR_IN_NANO).to_string() + "y";
        }
        if ns % WEEK_IN_NANO == 0 {
            return (ns / WEEK_IN_NANO).to_string() + "w";
        }
        if ns % DAY_IN_NANO == 0 {
            return (ns / DAY_IN_NANO).to_string() + "d";
        }
        if ns % HOUR_IN_NANO == 0 {
            return (ns / HOUR_IN_NANO).to_string() + "h";
        }
        if ns % MINUTE_IN_NANO == 0 {
            return (ns / MINUTE_IN_NANO).to_string() + "m";
        }
        if ns % SECOND_IN_NANO == 0 {
            return (ns / SECOND_IN_NANO).to_string() + "s";
        }
        if ns % MILLISECOND_IN_NANO == 0 {
            return (ns / MILLISECOND_IN_NANO).to_string() + "ms";
        }
        if ns % MICROSECOND_IN_NANO == 0 {
            return (ns / MICROSECOND_IN_NANO).to_string() + "us";
        }
        ns.to_string() + "ns"
    }
}

impl From<Duration> for DurationString {
    fn from(duration: Duration) -> Self {
        DurationString(duration)
    }
}

impl TryFrom<String> for DurationString {
    type Error = Error;

    fn try_from(duration: String) -> std::result::Result<Self, Self::Error> {
        duration.parse()
    }
}

impl FromStr for DurationString {
    type Err = Error;

    fn from_str(duration: &str) -> std::result::Result<Self, Self::Err> {
        let duration: Vec<char> = duration.chars().filter(|c| !c.is_whitespace()).collect();
        let mut grouped_durations: Vec<(Vec<char>, Vec<char>)> = vec![(vec![], vec![])];
        for i in 0..duration.len() {
            // Vector initialised with a starting element so unwraps should never panic
            if duration[i].is_numeric() {
                grouped_durations.last_mut().unwrap().0.push(duration[i]);
            } else {
                grouped_durations.last_mut().unwrap().1.push(duration[i]);
            }
            if i != duration.len() - 1 && !duration[i].is_numeric() && duration[i + 1].is_numeric()
            {
                // move to next group
                grouped_durations.push((vec![], vec![]));
            }
        }
        if grouped_durations.is_empty() {
            // `duration` either contains no numbers or no letters
            return Err(Error::Format);
        }
        let mut total_duration = Duration::new(0, 0);
        for (period, format) in grouped_durations {
            let period = match period.iter().collect::<String>().parse::<u64>() {
                Ok(period) => Ok(period),
                Err(err) => Err(Error::ParseInt(err)),
            }?;
            let multiply_period = |multiplier: u32| -> std::result::Result<Duration, Self::Err> {
                Duration::from_secs(period)
                    .checked_mul(multiplier)
                    .ok_or(Error::Overflow)
            };
            let period_duration = match format.iter().collect::<String>().as_ref() {
                "ns" => Ok(Duration::from_nanos(period)),
                "us" => Ok(Duration::from_micros(period)),
                "ms" => Ok(Duration::from_millis(period)),
                "s" => Ok(Duration::from_secs(period)),
                "m" => multiply_period(MINUTE_IN_SECONDS),
                "h" => multiply_period(HOUR_IN_SECONDS),
                "d" => multiply_period(DAY_IN_SECONDS),
                "w" => multiply_period(WEEK_IN_SECONDS),
                "y" => multiply_period(YEAR_IN_SECONDS),
                _ => Err(Error::Format),
            }?;
            total_duration = total_duration
                .checked_add(period_duration)
                .ok_or(Error::Overflow)?;
        }
        Ok(DurationString(total_duration))
    }
}

impl Deref for DurationString {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DurationString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Borrow<Duration> for DurationString {
    fn borrow(&self) -> &Duration {
        &self.0
    }
}

impl BorrowMut<Duration> for DurationString {
    fn borrow_mut(&mut self) -> &mut Duration {
        &mut self.0
    }
}

#[cfg(feature = "serde")]
struct DurationStringVisitor {
    marker: PhantomData<fn() -> DurationString>,
}

#[cfg(feature = "serde")]
impl DurationStringVisitor {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for DurationStringVisitor {
    type Value = DurationString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string")
    }

    fn visit_str<E>(self, string: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match DurationString::from_string(string.to_string()) {
            Ok(d) => Ok(d),
            Err(s) => Err(serde::de::Error::invalid_value(
                Unexpected::Str(&s.to_string()),
                &self,
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DurationString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DurationStringVisitor::new())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for DurationString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "serde")]
    #[derive(Serialize, Deserialize)]
    struct SerdeSupport {
        d: DurationString,
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialize_trait() {
        let s = SerdeSupport {
            d: DurationString::from_string(String::from("1m")).unwrap(),
        };
        assert_eq!(r#"{"d":"1m"}"#, serde_json::to_string(&s).unwrap());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deserialize_trait() {
        let s = r#"{"d":"2m"}"#;
        match serde_json::from_str::<SerdeSupport>(s) {
            Ok(v) => {
                assert_eq!(v.d.to_string(), "2m");
            }
            Err(err) => panic!("failed to deserialize: {}", err),
        }
    }

    #[test]
    fn test_string_int_overflow() {
        DurationString::from_string(String::from("ms")).expect_err("parsing \"ms\" should fail");
    }

    #[test]
    fn test_from_string_no_char() {
        DurationString::from_string(String::from("1234"))
            .expect_err("parsing \"1234\" should fail");
    }

    // fn test_from_string
    #[test]
    fn test_from_string() {
        let d = DurationString::from_string(String::from("100ms"));
        assert_eq!("100ms", format!("{}", d.unwrap()));
    }

    #[test]
    fn test_display_trait() {
        let d = DurationString::from(Duration::from_millis(100));
        assert_eq!("100ms", format!("{d}"));
    }

    #[test]
    fn test_from_duration() {
        let d: String = DurationString::from(Duration::from_millis(100)).into();
        assert_eq!(d, String::from("100ms"));
    }

    fn test_parse_string(input_str: &str, expected_duration: Duration) {
        let d_fromstr: Duration = input_str
            .parse::<DurationString>()
            .expect("Parse with FromStr failed")
            .into();
        assert_eq!(d_fromstr, expected_duration, "FromStr");
        let d_using_tryfrom: Duration = DurationString::try_from(input_str.to_owned())
            .expect("Parse with TryFrom failed")
            .into();
        assert_eq!(d_using_tryfrom, expected_duration, "TryFrom");
    }

    #[test]
    fn test_from_string_ms() {
        test_parse_string("100ms", Duration::from_millis(100));
    }

    #[test]
    fn test_from_string_us() {
        test_parse_string("100us", Duration::from_micros(100));
    }

    #[test]
    fn test_from_string_us_ms() {
        test_parse_string("1ms100us", Duration::from_micros(1100));
    }

    #[test]
    fn test_from_string_ns() {
        test_parse_string("100ns", Duration::from_nanos(100));
    }

    #[test]
    fn test_from_string_s() {
        test_parse_string("1s", Duration::from_secs(1));
    }

    #[test]
    fn test_from_string_m() {
        test_parse_string("1m", Duration::from_secs(60));
    }

    #[test]
    fn test_from_string_m_s() {
        test_parse_string("1m 1s", Duration::from_secs(61));
    }

    #[test]
    fn test_from_string_h() {
        test_parse_string("1h", Duration::from_secs(3600));
    }

    #[test]
    fn test_from_string_h_m() {
        test_parse_string("1h30m", Duration::from_secs(5400));
    }

    #[test]
    fn test_from_string_h_m2() {
        test_parse_string("1h128m", Duration::from_secs(11280));
    }

    #[test]
    fn test_from_string_d() {
        test_parse_string("1d", Duration::from_secs(86_400));
    }

    #[test]
    fn test_from_string_w() {
        test_parse_string("1w", Duration::from_secs(604_800));
    }

    #[test]
    fn test_from_string_w_s() {
        test_parse_string("1w 1s", Duration::from_secs(604_801));
    }

    #[test]
    fn test_from_string_y() {
        test_parse_string("1y", Duration::from_secs(31_556_926));
    }

    #[test]
    fn test_into_string_ms() {
        let d: String = DurationString::try_from(String::from("100ms"))
            .unwrap()
            .into();
        assert_eq!(d, "100ms");
    }

    #[test]
    fn test_into_string_s() {
        let d: String = DurationString::try_from(String::from("1s")).unwrap().into();
        assert_eq!(d, "1s");
    }

    #[test]
    fn test_into_string_m() {
        let d: String = DurationString::try_from(String::from("1m")).unwrap().into();
        assert_eq!(d, "1m");
    }

    #[test]
    fn test_into_string_h() {
        let d: String = DurationString::try_from(String::from("1h")).unwrap().into();
        assert_eq!(d, "1h");
    }

    #[test]
    fn test_into_string_d() {
        let d: String = DurationString::try_from(String::from("1d")).unwrap().into();
        assert_eq!(d, "1d");
    }

    #[test]
    fn test_into_string_w() {
        let d: String = DurationString::try_from(String::from("1w")).unwrap().into();
        assert_eq!(d, "1w");
    }

    #[test]
    fn test_into_string_y() {
        let d: String = DurationString::try_from(String::from("1y")).unwrap().into();
        assert_eq!(d, "1y");
    }

    #[test]
    fn test_into_string_overflow_unit() {
        let d: String = DurationString::try_from(String::from("1000ms"))
            .unwrap()
            .into();
        assert_eq!(d, "1s");

        let d: String = DurationString::try_from(String::from("60000ms"))
            .unwrap()
            .into();
        assert_eq!(d, "1m");

        let d: String = DurationString::try_from(String::from("61000ms"))
            .unwrap()
            .into();
        assert_eq!(d, "61s");
    }

    #[test]
    fn test_from_string_invalid_string() {
        DurationString::try_from(String::from("1000x"))
            .expect_err("Should have failed with invalid format");
    }

    #[test]
    fn test_try_from_string_overflow_y() {
        let result = DurationString::try_from(String::from("584554530873y"));
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_try_from_string_overflow_y_w() {
        let result = DurationString::try_from(String::from("584554530872y 29w"));
        assert_eq!(result, Err(Error::Overflow));
    }
}
