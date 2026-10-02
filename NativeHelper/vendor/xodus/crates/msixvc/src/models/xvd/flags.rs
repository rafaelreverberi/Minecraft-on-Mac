use msixvc_common::parse::byteorder::little_endian::*;
use msixvc_common::parse::{BinaryParse, BytesReader, EmptyReader};

use bitflags::bitflags;
use typenum::{U1 as T1, U2 as T2, U4 as T4};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XvdVolumeFlags: u32 {
        const READ_ONLY = 1 << 0;
        const ENCRYPTION_DISABLED = 1 << 1;
        const DATA_INTEGRITY_DISABLED = 1 << 2;
        const LEGACY_SECTOR_SIZE = 1 << 3;
        const RESILIENCY_ENABLED = 1 << 4;
        const SRA_READ_ONLY = 1 << 5;
        const REGION_ID_IN_XTS = 1 << 6;
        const ERA_SPECIFIC = 1 << 7;
    }
}

impl BinaryParse for XvdVolumeFlags {
    type Output = Self;
    type Size = T4;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<U32>();
        (XvdVolumeFlags::from_bits_retain(flags), r)
    }
}

impl XvdVolumeFlags {
    pub fn is_encrypted(&self) -> bool {
        !self.contains(Self::ENCRYPTION_DISABLED)
    }

    pub fn is_legacy_sector_size(&self) -> bool {
        self.contains(Self::LEGACY_SECTOR_SIZE)
    }

    pub fn is_data_integrity_enabled(&self) -> bool {
        !self.contains(Self::DATA_INTEGRITY_DISABLED)
    }

    pub fn is_resiliency_enabled(&self) -> bool {
        self.contains(Self::RESILIENCY_ENABLED)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WriteablePolicyFlags: u32 {}
}

impl BinaryParse for WriteablePolicyFlags {
    type Output = Self;
    type Size = T4;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<U32>();
        (WriteablePolicyFlags::from_bits_retain(flags), r)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XvcInfoFlags: u32 {}
}

impl BinaryParse for XvcInfoFlags {
    type Output = Self;
    type Size = T4;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<U32>();
        (XvcInfoFlags::from_bits_retain(flags), r)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XvcRegionFlags: u32 {
        const RESIDENT = 1 << 0;
        const INITIAL_PLAY = 1 << 1;
        const PREVIEW = 1 << 2;
        const FILE_SYSTEM_METADATA = 1 << 3;
        const PRESENT = 1 << 4;
        const ON_DEMAND = 1 << 5;
        const AVAILABLE = 1 << 6;
    }
}

impl BinaryParse for XvcRegionFlags {
    type Output = Self;
    type Size = T4;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<U32>();
        (XvcRegionFlags::from_bits_retain(flags), r)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XvcRegionPresenceInfoFlags: u8 {
        const IS_PRESENT = 1 << 0;
        const IS_AVAILABLE = 1 << 1;
    }
}

impl BinaryParse for XvcRegionPresenceInfoFlags {
    type Output = Self;
    type Size = T1;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<u8>();
        (XvcRegionPresenceInfoFlags::from_bits_retain(flags), r)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XvdSegmentMetadataSegmentFlags: u16 {
        const KEEP_ENCRYPTED_ON_DISK = 1;
    }
}

impl BinaryParse for XvdSegmentMetadataSegmentFlags {
    type Output = Self;
    type Size = T2;

    fn parse<'a>(r: BytesReader<'a, Self::Size>) -> (Self::Output, EmptyReader<'a>) {
        let (flags, r) = r.read::<U16>();
        (XvdSegmentMetadataSegmentFlags::from_bits_retain(flags), r)
    }
}
