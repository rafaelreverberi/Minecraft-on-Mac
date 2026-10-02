use msixvc_common::parse::byteorder::little_endian::*;
use msixvc_common::parse::structs::Version;
use msixvc_common::parse::{BinaryTryParse, BytesReader, EmptyReader};

use typenum::{U16 as T16, U860 as T860};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct XspHeader {
    pub content_id: uuid::Uuid,
    pub plan_id: uuid::Uuid,
    pub xsp_id: uuid::Uuid,
    pub page_size: u32,
    pub record_count: u32,
    pub total_download: u64,
    pub disk_space_required: u64,
    pub upgrade_from_version: Version,
    pub upgrade_to_version: Version,
}

impl XspHeader {
    const MAGIC: &[u8; 8] = b"MS-XPFM ";
}

#[derive(thiserror::Error, Debug)]
pub enum XspHeaderParseError {
    #[error(r#"invalid magic: expected {magic:?}, got {0:?}"#, magic = XspHeader::MAGIC)]
    InvalidMagic([u8; 8]),
}

impl BinaryTryParse for XspHeader {
    type Output = Self;
    type Size = T860;
    type Error = XspHeaderParseError;

    fn try_parse<'a>(
        r: BytesReader<'a, Self::Size>,
    ) -> Result<(Self::Output, EmptyReader<'a>), Self::Error> {
        let (_signature, r) = r.array::<0x200>();

        let r = r.magic(Self::MAGIC).map_err(Self::Error::InvalidMagic)?;

        let (block_size_or_payload, r) = r.read::<U32>(); // Or payload offset
        let (_unknown_val, r) = r.array::<4>();
        let (vduid, r) = r.read::<Uuid>();
        let (_uduid, r) = r.read::<Uuid>();
        let (_build_id, r) = r.read::<Uuid>();
        let (_reserved, r) = r.array::<0x30>();
        let (_unknown1, r) = r.read::<U32>();
        let (_unknown2, r) = r.read::<U32>();
        let (_unknown3, r) = r.read::<U32>();
        let (record_count, r) = r.read::<U32>();
        let (_unknown_block_size_or_payload, r) = r.read::<U64>();
        let (_reserved2, r) = r.array::<8>();
        let (_reserved3, r) = r.array::<8>();
        let (_reserved4, r) = r.array::<8>();
        let (_reserved5, r) = r.array::<8>();
        let (_unknown_int1, r) = r.read::<U64>();
        let (_next_block_size, r) = r.read::<U64>();
        let (_unknown4, r) = r.read::<U64>();
        let (_number_of_elements, r) = r.read::<U32>();
        let (_value_1, r) = r.read::<U32>();
        let (total_bytes, r) = r.read::<U64>();
        let (disk_space_required, r) = r.read::<U64>();
        let (_value_0, r) = r.read::<U64>();
        let (_unknown5, r) = r.read::<U64>();
        let (_value2_0, r) = r.read::<U64>();
        let (_unknown_big_value, r) = r.read::<U64>();
        let (_unknown6, r) = r.read::<U64>();
        let (_always_64, r) = r.read::<U64>(); // Potential alignment / cluster size
        let (_reserved6, r) = r.array::<0x10>();
        let (plan_id, r) = r.read::<Uuid>();
        let (_value3_0, r) = r.array::<0x14>();
        let (xsp_id, r) = r.read::<Uuid>();
        let (previous_build_version, r) = r.read::<Version>();
        let (current_build_version, r) = r.read::<Version>();

        Ok((
            Self {
                content_id: vduid,
                plan_id,
                xsp_id,
                page_size: block_size_or_payload,
                record_count,
                total_download: total_bytes,
                disk_space_required,
                upgrade_from_version: previous_build_version,
                upgrade_to_version: current_build_version,
            },
            r,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum XspPatchRecord {
    NewData {
        block_number: u32,
        block_count: u32,
    },
    CopyData {
        old_block_number: u32,
        new_block_number: u32,
        block_count: u32,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum XspPatchRecordParseError {
    #[error("Unknown patch record flag {0:X}")]
    UnknownFlag(u32),
}

impl BinaryTryParse for XspPatchRecord {
    type Output = Self;
    type Size = T16;
    type Error = XspPatchRecordParseError;

    fn try_parse<'a>(
        r: BytesReader<'a, Self::Size>,
    ) -> Result<(Self::Output, EmptyReader<'a>), Self::Error> {
        let (source_offset, r) = r.read::<U32>();
        let (flag, r) = r.read::<U32>();
        let (target_offset, r) = r.read::<U32>();
        let (block_count, r) = r.read::<U32>();

        Ok((
            match flag {
                0 => Self::NewData {
                    block_number: target_offset,
                    block_count,
                },
                0x88000000 => Self::CopyData {
                    old_block_number: source_offset,
                    new_block_number: target_offset,
                    block_count,
                },
                _ => return Err(Self::Error::UnknownFlag(flag)),
            },
            r,
        ))
    }
}
