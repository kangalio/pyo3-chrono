#![warn(missing_docs)]

//! This crate provides newtype wrappers around chrono's [`NaiveDateTime`], [`NaiveDate`],
//! [`NaiveTime`], and [`Duration`] structs, that can be used in [`PyO3`](pyo3) applications.
//!
//! Leap seconds are handled correctly, however timezones are not supported because Python itself
//! doesn't inherently support timezones in its datetimes.
//!
//! Implementations for the [`serde::Serialize`] and [`serde::Deserialize`] traits can be enabled via the
//! `serde` feature flag.
//!
//! # Truncation
//! Python can store durations from negative one billion days up to positive one billion days long,
//! in microsecond precision. However,
//! Chrono only accepts microseconds as i64:
//! ```text
//! Python's max duration: 84599999999999999999 microseconds
//! Chrono's max duration: 9223372036854775807 microseconds
//!
//! Python's min duration: -84599999915400000000 microseconds
//! Chrono's min duration: -9223372036854775808 microseconds
//! ```
//! As you can see, Chrono doesn't support the entire range of durations that Python supports.
//! When encountering durations that are unrepresentable in Chrono, this library truncates the
//! duration to the nearest supported duration.

pub use chrono;
pub use pyo3;
#[cfg(feature = "serde")]
pub use serde_ as serde;

use chrono::{Datelike as _, Timelike as _};
use pyo3::types::{PyDateAccess as _, PyDeltaAccess as _, PyTimeAccess as _};
use std::convert::TryInto as _;

fn chrono_to_micros_and_fold(time: impl chrono::Timelike) -> (u32, bool) {
    if let Some(folded_nanos) = time.nanosecond().checked_sub(1_000_000_000) {
        (folded_nanos / 1000, true)
    } else {
        (time.nanosecond() / 1000, false)
    }
}

fn py_to_micros(time: &impl pyo3::types::PyTimeAccess) -> u32 {
    if time.get_fold() {
        time.get_microsecond() + 1_000_000
    } else {
        time.get_microsecond()
    }
}

macro_rules! new_type {
    ($doc:literal, $name:ident, $inner_type:ty) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(pub $inner_type);

        impl From<$inner_type> for $name {
            fn from(inner: $inner_type) -> Self {
                Self(inner)
            }
        }

        impl From<$name> for $inner_type {
            fn from(wrapper: $name) -> Self {
                wrapper.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

macro_rules! impl_serde_traits {
    ($new_type:ty, $inner_type:ty) => {
        #[cfg(feature = "serde")]
        impl serde::Serialize for $new_type {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $new_type {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                <$inner_type>::deserialize(deserializer).map(Self)
            }
        }
    };
}

new_type!(
    "A wrapper around [`chrono::NaiveDateTime`] that can be converted to and from Python's `datetime.datetime`",
    NaiveDateTime,
    chrono::NaiveDateTime
);
impl_serde_traits!(NaiveDateTime, chrono::NaiveDateTime);

impl pyo3::ToPyObject for NaiveDateTime {
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        let (micros, fold) = chrono_to_micros_and_fold(self.0);
        pyo3::types::PyDateTime::new_with_fold(
            py,
            self.0.year(),
            self.0.month() as u8,
            self.0.day() as u8,
            self.0.hour() as u8,
            self.0.minute() as u8,
            self.0.second() as u8,
            micros,
            None,
            fold,
        )
        .unwrap()
        .to_object(py)
    }
}

impl pyo3::IntoPy<pyo3::PyObject> for NaiveDateTime {
    fn into_py(self, py: pyo3::Python) -> pyo3::PyObject {
        pyo3::ToPyObject::to_object(&self, py)
    }
}

impl pyo3::FromPyObject<'_> for NaiveDateTime {
    fn extract(ob: &pyo3::PyAny) -> pyo3::PyResult<Self> {
        let pydatetime: &pyo3::types::PyDateTime = pyo3::PyTryFrom::try_from(ob)?;
        Ok(NaiveDateTime(
            chrono::NaiveDate::from_ymd(
                pydatetime.get_year(),
                pydatetime.get_month() as u32,
                pydatetime.get_day() as u32,
            )
            .and_hms_micro(
                pydatetime.get_hour() as u32,
                pydatetime.get_minute() as u32,
                pydatetime.get_second() as u32,
                py_to_micros(pydatetime),
            ),
        ))
    }
}

new_type!(
    "A wrapper around [`chrono::NaiveDate`] that can be converted to and from Python's `datetime.date`",
    NaiveDate,
    chrono::NaiveDate
);
impl_serde_traits!(NaiveDate, chrono::NaiveDate);

impl pyo3::ToPyObject for NaiveDate {
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        pyo3::types::PyDate::new(py, self.0.year(), self.0.month() as u8, self.0.day() as u8)
            .unwrap()
            .to_object(py)
    }
}

impl pyo3::IntoPy<pyo3::PyObject> for NaiveDate {
    fn into_py(self, py: pyo3::Python) -> pyo3::PyObject {
        pyo3::ToPyObject::to_object(&self, py)
    }
}

impl pyo3::FromPyObject<'_> for NaiveDate {
    fn extract(ob: &pyo3::PyAny) -> pyo3::PyResult<Self> {
        let pydate: &pyo3::types::PyDate = pyo3::PyTryFrom::try_from(ob)?;
        Ok(NaiveDate(chrono::NaiveDate::from_ymd(
            pydate.get_year(),
            pydate.get_month() as u32,
            pydate.get_day() as u32,
        )))
    }
}

new_type!(
    "A wrapper around [`chrono::NaiveTime`] that can be converted to and from Python's `datetime.time`",
    NaiveTime,
    chrono::NaiveTime
);
impl_serde_traits!(NaiveTime, chrono::NaiveTime);

impl pyo3::ToPyObject for NaiveTime {
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        let (micros, fold) = chrono_to_micros_and_fold(self.0);
        pyo3::types::PyTime::new_with_fold(
            py,
            self.0.hour() as u8,
            self.0.minute() as u8,
            self.0.second() as u8,
            micros,
            None,
            fold,
        )
        .unwrap()
        .to_object(py)
    }
}

impl pyo3::IntoPy<pyo3::PyObject> for NaiveTime {
    fn into_py(self, py: pyo3::Python) -> pyo3::PyObject {
        pyo3::ToPyObject::to_object(&self, py)
    }
}

impl pyo3::FromPyObject<'_> for NaiveTime {
    fn extract(ob: &pyo3::PyAny) -> pyo3::PyResult<Self> {
        let pytime: &pyo3::types::PyTime = pyo3::PyTryFrom::try_from(ob)?;
        Ok(NaiveTime(chrono::NaiveTime::from_hms_micro(
            pytime.get_hour() as u32,
            pytime.get_minute() as u32,
            pytime.get_second() as u32,
            py_to_micros(pytime),
        )))
    }
}

new_type!(
    "A wrapper around [`chrono::Duration`] that can be converted to and from Python's `datetime.timedelta`",
    Duration,
    chrono::Duration
);
// impl_serde_traits!(Duration, chrono::Duration); // chrono doesn't yet support serde traits for it

impl pyo3::ToPyObject for Duration {
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        const MICROSECONDS_PER_DAY: i64 = 60 * 60 * 24 * 1_000_000;

        // There's a lot of clamping involved here because chrono doesn't expose enough
        // functionality for clean 1:1 conversions
        let total_micros = self.0.num_microseconds().unwrap_or(i64::MAX);
        let total_days = (total_micros / MICROSECONDS_PER_DAY)
            .try_into()
            .unwrap_or(i32::MAX);
        // We can safely cast to i32 because we moduloed and therefore must be in i32 bounds
        let subday_micros = (total_micros % MICROSECONDS_PER_DAY) as i32;

        // We can pass zero for seconds here because we contain the seconds in subday_micros,
        // and because we pass true, Python normalizes the given values anyways: "Normalization is
        // performed so that the resulting number of microseconds and seconds lie in the ranges
        // documented for datetime.timedelta objects."
        pyo3::types::PyDelta::new(py, total_days, 0, subday_micros, true)
            .unwrap()
            .into()
    }
}

impl pyo3::IntoPy<pyo3::PyObject> for Duration {
    fn into_py(self, py: pyo3::Python) -> pyo3::PyObject {
        pyo3::ToPyObject::to_object(&self, py)
    }
}

impl pyo3::FromPyObject<'_> for Duration {
    fn extract(ob: &pyo3::PyAny) -> pyo3::PyResult<Self> {
        let pydelta: &pyo3::types::PyDelta = pyo3::PyTryFrom::try_from(ob)?;

        let total_days = pydelta.get_days() as i64;
        let total_seconds = total_days * 24 * 60 * 60 + pydelta.get_seconds() as i64;
        let total_microseconds = total_seconds
            .saturating_mul(1_000_000)
            .saturating_add(pydelta.get_microseconds() as i64);

        Ok(Duration(chrono::Duration::microseconds(total_microseconds)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::ToPyObject as _;

    /// Assert that a Python-native object is equal to a converted Rust object
    fn assert_py_eq(
        native_python_object: &(impl pyo3::PyNativeType + pyo3::ToPyObject + std::fmt::Display),
        // foreign_object: &(impl pyo3::ToPyObject + std::fmt::Display),
        foreign_object: &pyo3::PyObject,
    ) {
        // Cast python object to PyAny because PyO3 only implements comparing for PyAny
        // Then, actually compare against foreign object
        if native_python_object
            .to_object(native_python_object.py())
            .cast_as::<pyo3::PyAny>(native_python_object.py())
            .unwrap()
            .compare(foreign_object)
            .unwrap()
            != std::cmp::Ordering::Equal
        {
            panic!(
                r#"assertion failed: converted Rust object is not equal to Python reference object
         Python: `{}`
 Converted Rust: `{}`"#,
                native_python_object, foreign_object,
            );
        }
    }

    #[test]
    fn test_datetime() {
        let py = pyo3::Python::acquire_gil();
        let py = py.python();

        for &(year, month, day, hour, min, sec, micro, is_leap) in &[
            (2021, 1, 20, 22, 39, 46, 186605, false), // time of writing :)
            (2020, 2, 29, 0, 0, 0, 0, false),         // leap day hehe
            (2016, 12, 31, 23, 59, 59, 123456, false), // latest leap second
            (2016, 12, 31, 23, 59, 59, 123456, true), // latest leap second
            (1156, 3, 31, 11, 22, 33, 445566, false), // long ago
            (1, 1, 1, 0, 0, 0, 0, false),             // Jan 01, 1 AD - can't go further than this
            (3000, 6, 5, 4, 3, 2, 1, false),          // the future
            (9999, 12, 31, 23, 59, 59, 999999, false), // Dec 31, 9999 AD - can't go further than this
        ] {
            // Check if date conversion works

            let py_date = pyo3::types::PyDate::new(py, year, month, day).unwrap();
            let chrono_date =
                NaiveDate(chrono::NaiveDate::from_ymd(year, month.into(), day.into()));

            assert_eq!(py_date.extract::<NaiveDate>().unwrap(), chrono_date);
            assert_py_eq(py_date, &chrono_date.to_object(py));

            // Check if time conversion works

            let py_time =
                pyo3::types::PyTime::new_with_fold(py, hour, min, sec, micro, None, is_leap)
                    .unwrap();
            let chrono_time = NaiveTime(chrono::NaiveTime::from_hms_micro(
                hour.into(),
                min.into(),
                sec.into(),
                micro + if is_leap { 1_000_000 } else { 0 },
            ));

            assert_eq!(py_time.extract::<NaiveTime>().unwrap(), chrono_time);
            assert_py_eq(py_time, &chrono_time.to_object(py));

            // Check if datetime conversion works

            let py_datetime = pyo3::types::PyDateTime::new_with_fold(
                py, year, month, day, hour, min, sec, micro, None, is_leap,
            )
            .unwrap();
            let chrono_datetime =
                NaiveDateTime(chrono::NaiveDateTime::new(chrono_date.0, chrono_time.0));

            assert_eq!(
                py_datetime.extract::<NaiveDateTime>().unwrap(),
                chrono_datetime
            );
            assert_py_eq(py_datetime, &chrono_datetime.to_object(py))
        }
    }

    #[test]
    fn test_duration() {
        let py = pyo3::Python::acquire_gil();
        let py = py.python();

        for &(days, seconds, micros, total_micros, test_to_python_conversion) in &[
            (0, 0, 0, 0, true),
            (0, 0, 1, 1, true),
            (0, 0, -1, -1, true),
            (156, 32, 415178, 13478432415178, true),
            (-10000, 0, 0, -864000000000000, true),
            (0, 0, 999_999, 999_999, true),
            (0, 0, 1_000_000, 1_000_000, true),
            (0, 36 * 60, 0, 36 * 60 * 1_000_000, true),
            (0, 0, i32::MAX, i32::MAX as i64, true),
            (0, 0, i32::MIN, i32::MIN as i64, true),
            (
                // Python's max duration is 1 billion days...
                999999999,
                86399,
                999999,
                // ...which is not representable in Chrono, hence our library aims to clamp to the
                // nearest value:
                i64::MAX,
                // Don't check if Chrono conversion to Python fails - it will definitely fail
                // because Chrono's truncated duration doesn't match Python's full duration
                false,
            ),
            (
                // Python's max duration is negative 1 billion days...
                -999999999,
                0,
                0,
                // ...which is not representable in Chrono, hence our library aims to clamp to the
                // nearest value when converting from Python to Rust:
                i64::MIN,
                // Don't check if Chrono conversion to Python fails - it will definitely fail
                // because Chrono's truncated duration doesn't match Python's full duration
                false,
            ),
        ] {
            let py_duration = pyo3::types::PyDelta::new(py, days, seconds, micros, true).unwrap();
            let chrono_duration = Duration(chrono::Duration::microseconds(total_micros));

            assert_eq!(py_duration.extract::<Duration>().unwrap(), chrono_duration);
            if test_to_python_conversion {
                assert_py_eq(py_duration, &chrono_duration.to_object(py));
            }
        }
    }
}
