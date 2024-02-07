use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use codepage_437::{FromCp437, CP437_CONTROL};

use crate::constants::CENTRAL_DIRECTOR_BEGIN_SIGNATURE;
#[derive(Debug)]
pub struct CentralDirectory {
    pub relative_offset: u32,
    pub file_name: String,
    pub extra_field: Vec<u8>,
    pub file_comment: String,
}

impl CentralDirectory {
    pub fn parse<T: Read + Seek>(reader: &mut T) -> std::io::Result<Self> {
        if reader.read_u32::<LittleEndian>()? != CENTRAL_DIRECTOR_BEGIN_SIGNATURE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid Central Directory Begin Signature",
            ));
        }

        reader.seek(SeekFrom::Current(24))?;
        let file_name_length = reader.read_u16::<LittleEndian>()?;
        let extra_field_length = reader.read_u16::<LittleEndian>()?;
        let file_comment_length = reader.read_u16::<LittleEndian>()?;
        reader.seek(SeekFrom::Current(8))?;
        let relative_offset = reader.read_u32::<LittleEndian>()?;

        let mut name_file = vec![0; file_name_length as usize];
        reader.read_exact(&mut name_file)?;

        let mut extra_field = vec![0; extra_field_length as usize];
        reader.read_exact(&mut extra_field)?;

        let mut file_comment = vec![0; file_comment_length as usize];
        reader.read_exact(&mut file_comment)?;

        Ok(Self {
            relative_offset,
            file_name: String::from_cp437(name_file, &CP437_CONTROL),
            extra_field: extra_field,
            file_comment: String::from_cp437(file_comment, &CP437_CONTROL),
        })
    }
}
