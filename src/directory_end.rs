use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::constants::CENTRAL_DIRECTOR_END_SIGNATURE;

#[derive(Debug)]
pub struct CentralDirectorEnd {
    pub disk_number: u16,
    pub disk_number_with_central_directory: u16,
    pub number_of_files: u16,
    pub total_number_of_files: u16,
    pub central_directory_size: u32,
    pub central_directory_offset: u32,
    pub comment: Vec<u8>,
}

impl CentralDirectorEnd {
    pub fn parse<T: Read>(reader: &mut T) -> std::io::Result<Self> {
        if reader.read_u32::<LittleEndian>()? != CENTRAL_DIRECTOR_END_SIGNATURE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid Central Directory End Signature",
            ));
        }

        let disk_number = reader.read_u16::<LittleEndian>()?;
        let disk_number_with_central_directory = reader.read_u16::<LittleEndian>()?;
        let number_of_files = reader.read_u16::<LittleEndian>()?;
        let total_number_of_files = reader.read_u16::<LittleEndian>()?;
        let central_directory_size = reader.read_u32::<LittleEndian>()?;
        let central_directory_offset = reader.read_u32::<LittleEndian>()?;
        let comment_length = reader.read_u16::<LittleEndian>()?;
        let mut comment = vec![0; comment_length as usize];

        reader.read_exact(&mut comment)?;

        Ok(Self {
            disk_number,
            disk_number_with_central_directory,
            number_of_files,
            total_number_of_files,
            central_directory_size,
            central_directory_offset,
            comment,
        })
    }

    pub fn find_and_parse<T: Read + Seek>(
        reader: &mut T,
    ) -> std::io::Result<(CentralDirectorEnd, u64)> {
        const HEADER_SIZE: u64 = 22;
        let file_length = reader.seek(SeekFrom::End(0))?;

        let search_upper_bound = file_length.saturating_sub(HEADER_SIZE + ::std::u16::MAX as u64);

        if file_length < HEADER_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "File too small to contain End of Central Directory",
            ));
        }

        let mut position = file_length - HEADER_SIZE;

        while position >= search_upper_bound {
            reader.seek(SeekFrom::Start(position))?;

            if reader.read_u32::<LittleEndian>()? == CENTRAL_DIRECTOR_END_SIGNATURE {
                let cde_start_position = reader.seek(SeekFrom::Start(position))?;
                return Ok((CentralDirectorEnd::parse(reader)?, cde_start_position));
            }

            position = match position.checked_sub(1) {
                Some(position) => position,
                None => break,
            };
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "End of Central Directory not found",
        ))
    }
}
