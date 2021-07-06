# pyo3-chrono
This crate provides newtype wrappers around chrono's `NaiveDateTime`, `NaiveDate`,
`NaiveTime`, and `Duration` structs, that can be used in `PyO3` applications.

Leap seconds are handled correctly, however timezones are not supported because Python itself
doesn't inherently support timezones in its datetimes.

Implementations for the `serde::Serialize` and `serde::Deserialize` traits can be enabled via the
`serde` feature flag.

## Truncation
Python can store durations from negative one billion days up to positive one billion days long,
in microsecond precision. However,
Chrono only accepts microseconds as i64:
```
Python's max duration: 84599999999999999999 microseconds
Chrono's max duration: 9223372036854775807 microseconds

Python's min duration: -84599999915400000000 microseconds
Chrono's min duration: -9223372036854775808 microseconds
```
As you can see, Chrono doesn't support the entire range of durations that Python supports.
When encountering durations that are unrepresentable in Chrono, this library truncates the
duration to the nearest supported duration.