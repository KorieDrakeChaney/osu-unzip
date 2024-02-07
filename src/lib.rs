use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};
use codepage_437::{FromCp437, CP437_CONTROL};
use constants::CENTRAL_DIRECTOR_BEGIN_SIGNATURE;
use flate2::read::DeflateDecoder;

use crate::constants::CENTRAL_DIRECTOR_END_SIGNATURE;

mod constants;

fn find_directory_end<T: Read + Seek>(reader: &mut T) -> std::io::Result<u64> {
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
            return Ok(cde_start_position);
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

fn save_file<T: Read + Seek>(reader: &mut T) -> std::io::Result<()> {
    if reader.read_u32::<LittleEndian>()? != constants::LOCAL_FILE_HEADER_SIGNATURE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid Local File Header Signature",
        ));
    }

    reader.seek(SeekFrom::Current(14))?;
    let compressed_size = reader.read_u32::<LittleEndian>()?;
    reader.seek(SeekFrom::Current(4))?;
    let file_name_length = reader.read_u16::<LittleEndian>()?;
    let extra_field_length = reader.read_u16::<LittleEndian>()?;
    let mut file_name = vec![0; file_name_length as usize];
    reader.read_exact(&mut file_name)?;
    let mut extra_field = vec![0; extra_field_length as usize];
    reader.read_exact(&mut extra_field)?;

    let mut data_buffer = vec![0; compressed_size as usize];

    reader.read_exact(&mut data_buffer)?;

    let mut decoder = DeflateDecoder::new(&data_buffer[..]);

    let mut decompressed_data = Vec::new();

    decoder.read_to_end(&mut decompressed_data)?;

    let mut file = std::fs::File::create(String::from_cp437(file_name, &CP437_CONTROL))?;

    file.write_all(&decompressed_data)?;

    Ok(())
}

pub fn parse_osz(file: &str) -> std::io::Result<()> {
    let reader = &mut std::fs::File::open(file)?;

    let central_directory_end = find_directory_end(reader)?;

    reader.seek(SeekFrom::Start(central_directory_end))?;

    reader.seek(SeekFrom::Current(16))?;

    let central_directory_offset = reader.read_u32::<LittleEndian>()?;

    reader.seek(SeekFrom::Start(central_directory_offset as u64))?;

    loop {
        if reader.read_u32::<LittleEndian>().unwrap() == 0x06054b50 {
            break;
        } else {
            reader.seek(SeekFrom::Current(-4)).unwrap();
        }

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

        let relative_offset = reader.read_u32::<LittleEndian>().unwrap();

        let start_position = reader
            .seek(SeekFrom::Current(
                (0 + file_name_length + file_comment_length + extra_field_length) as i64,
            ))
            .unwrap();

        reader
            .seek(SeekFrom::Start(relative_offset as u64))
            .unwrap();

        save_file(reader).unwrap();

        reader.seek(SeekFrom::Start(start_position)).unwrap();
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        parse_osz("./test.osz").unwrap();
    }
}
