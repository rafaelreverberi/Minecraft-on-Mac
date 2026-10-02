//! Additional types for parsing with a [`BytesReader`].
//!
//! The implementations of [`BinaryParse`] for these types are not intended for
//! general use, as they provide only one of the many ways to parse each type,
//! the one used in `MSIXVC` binaries (for example, a UUID can be parsed in
//! little-endian vs big-endian).

use super::byteorder::little_endian::{I64 as LeI64, U16 as LeU16};
use super::{BinaryParse, BytesReader, EmptyReader};

use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt::{self, Debug, Display};

use chrono::DateTime;
use typenum::{U8, U16};
use uuid::Uuid;

/// A version number that consists of major, minor, patch, and build components.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u16,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(self.build.cmp(&other.build))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the Display implementation as the Debug one
        write!(f, "{}", self)
    }
}

impl BinaryParse for Version {
    type Output = Version;
    type Size = U8;

    #[inline]
    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (build, r) = r.read::<LeU16>();
        let (patch, r) = r.read::<LeU16>();
        let (minor, r) = r.read::<LeU16>();
        let (major, r) = r.read::<LeU16>();

        (
            Version {
                build,
                patch,
                minor,
                major,
            },
            r,
        )
    }
}

impl BinaryParse for Uuid {
    type Output = Uuid;
    type Size = U16;

    #[inline]
    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (&uuid, r) = r.remaining();
        (Uuid::from_bytes_le(uuid.into_array()), r)
    }
}

/// Converts a Microsoft FILETIME (number of 100ns intervals since 1601-01-01 UTC)
/// into a [`chrono::DateTime`]
#[inline]
const fn microsoft_filetime(filetime: i64) -> DateTime<chrono::Utc> {
    // FILETIME counts 100ns intervals since 1601-01-01 UTC.
    // Unix time counts nanoseconds since 1970-01-01 UTC.

    /// Number of 100 nanoseconds between FILETIME epoch and Unix time
    const FILETIME_TO_UNIX: i64 = 116_444_736_000_000_000;

    let unix_nanos = (filetime - FILETIME_TO_UNIX) * 100;
    DateTime::from_timestamp_nanos(unix_nanos)
}

/// A marker type that implements [`BinaryParse`] for parsing a Microsoft FILETIME
/// into a [`chrono`]'s [`DateTime<Utc>`].
pub struct Filetime;

impl BinaryParse for Filetime {
    type Output = DateTime<chrono::Utc>;
    type Size = U8;

    #[inline]
    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (filetime, r) = r.read::<LeI64>();
        (microsoft_filetime(filetime), r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_cmp() {
        let lower = Version {
            major: 1,
            minor: 26,
            patch: 3005,
            build: 0,
        };
        let higher = Version {
            major: 1,
            minor: 26,
            patch: 3101,
            build: 0,
        };
        let other_high = Version {
            major: 1,
            minor: 26,
            patch: 3101,
            build: 0,
        };
        let other_high2 = Version {
            major: 2,
            minor: 26,
            patch: 3101,
            build: 0,
        };

        assert!(lower < higher);
        assert!(higher > lower);
        assert!(higher == other_high);
        assert!(other_high2 > other_high);
    }
}
