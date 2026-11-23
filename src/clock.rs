use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

/// The instant `AGPJ_FIXED_NOW` pins the board to, for example `2026-11-02T09:00:00Z`; `None` when it is unset or
/// does not parse. The sample-media scripts set it so every capture shows the same dates.
pub fn fixed_now() -> Option<DateTime<Utc>> {
    static FIXED: std::sync::OnceLock<Option<DateTime<Utc>>> = std::sync::OnceLock::new();
    *FIXED.get_or_init(|| {
        std::env::var("AGPJ_FIXED_NOW")
            .ok()
            .and_then(|value| parse_utc(&value).ok())
    })
}

/// The current instant, or the one `AGPJ_FIXED_NOW` pins. Product code takes its time from here instead of
/// calling `Utc::now()`.
pub fn now() -> DateTime<Utc> {
    fixed_now().unwrap_or_else(Utc::now)
}

/// A clock that always returns the same instant. For tests.
pub struct FixedClock(pub DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

const UTC_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Format as UTC RFC 3339 with second precision, e.g. `2026-10-02T01:08:00Z`.
pub fn format_utc(dt: DateTime<Utc>) -> String {
    dt.format(UTC_FORMAT).to_string()
}

/// Parse a `Z`-suffixed UTC timestamp with second precision. Offsets such as
/// `+09:00` are rejected.
pub fn parse_utc(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    NaiveDateTime::parse_from_str(s, UTC_FORMAT).map(|naive| naive.and_utc())
}

/// Format in the local time zone as `MM/DD HH:mm`, e.g. `10/02 10:08`.
pub fn format_local_short(dt: DateTime<Utc>) -> String {
    format_short_in(dt, &Local)
}

/// Format in the given time zone as `MM/DD HH:mm`. Tests pass a fixed zone.
fn format_short_in<Tz>(dt: DateTime<Utc>, tz: &Tz) -> String
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    dt.with_timezone(tz).format("%m/%d %H:%M").to_string()
}

/// serde adapter: `#[serde(with = "agent_project::clock::serde_utc")]`.
pub mod serde_utc {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::format_utc(*dt))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let text = String::deserialize(d)?;
        super::parse_utc(&text).map_err(D::Error::custom)
    }
}

pub mod serde_opt_utc {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    pub fn serialize<S: Serializer>(dt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        dt.map(super::format_utc).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|text| super::parse_utc(&text).map_err(D::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn sample() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 2, 1, 8, 0).unwrap()
    }

    #[test]
    fn format_utc_uses_second_precision_and_z() {
        assert_eq!(format_utc(sample()), "2026-10-02T01:08:00Z");
    }

    #[test]
    fn format_utc_drops_sub_second_precision() {
        let dt = sample() + chrono::Duration::milliseconds(789);
        assert_eq!(format_utc(dt), "2026-10-02T01:08:00Z");
    }

    #[test]
    fn parse_utc_accepts_z_suffixed_rfc3339() {
        assert_eq!(parse_utc("2026-10-02T01:08:00Z").unwrap(), sample());
    }

    #[test]
    fn parse_utc_rejects_offsets_and_garbage() {
        assert!(parse_utc("2026-10-02T10:08:00+09:00").is_err());
        assert!(parse_utc("2026-10-02T01:08:00+00:00").is_err());
        assert!(parse_utc("2026-10-02 01:08:00").is_err());
        assert!(parse_utc("2026-10-02T01:08:00").is_err());
        assert!(parse_utc("2026-10-02T01:08:00z").is_err());
        assert!(parse_utc("not a date").is_err());
        assert!(parse_utc("").is_err());
    }

    #[test]
    fn format_and_parse_round_trip() {
        assert_eq!(parse_utc(&format_utc(sample())).unwrap(), sample());
    }

    #[test]
    fn fixed_clock_returns_its_instant() {
        assert_eq!(FixedClock(sample()).now(), sample());
    }

    #[test]
    fn system_clock_is_close_to_now() {
        let before = Utc::now();
        let now = SystemClock.now();
        let after = Utc::now();
        assert!(before <= now && now <= after);
    }

    #[test]
    fn local_short_formats_month_day_hour_minute() {
        assert_eq!(format_short_in(sample(), &Utc), "10/02 01:08");
        let tokyo = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        assert_eq!(format_short_in(sample(), &tokyo), "10/02 10:08");
    }

    #[test]
    fn serde_adapter_round_trips_through_json() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Wrap(#[serde(with = "serde_utc")] DateTime<Utc>);
        let json = serde_json::to_string(&Wrap(sample())).unwrap();
        assert_eq!(json, "\"2026-10-02T01:08:00Z\"");
        assert_eq!(serde_json::from_str::<Wrap>(&json).unwrap(), Wrap(sample()));
        assert!(serde_json::from_str::<Wrap>("\"2026-10-02T10:08:00+09:00\"").is_err());
    }
}
