use msixvc_common::parse::BinaryTryParse;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, BufReader};

use crate::models::xsp::{XspHeader, XspPatchRecord};

pub struct XspFile {
    pub header: XspHeader,
    pub entries: Vec<XspPatchRecord>,
}

impl XspFile {
    pub async fn parse_file<Reader>(file: Reader) -> Result<Self, Box<dyn std::error::Error>>
    where
        Reader: AsyncRead + AsyncSeek + Unpin,
    {
        let mut file = BufReader::new(file);

        let header = {
            let mut buf = XspHeader::buffer();
            file.read_exact(&mut buf).await?;
            XspHeader::try_from_array(&buf)?
        };

        let mut entries = Vec::with_capacity(header.record_count as usize);
        file.seek(std::io::SeekFrom::Start(header.page_size as u64))
            .await?;

        let mut buf = XspPatchRecord::buffer();

        for _ in 0..header.record_count {
            file.read_exact(&mut buf).await?;
            let record = XspPatchRecord::try_from_array(&buf)?;
            entries.push(record);
        }

        Ok(Self { header, entries })
    }
}
