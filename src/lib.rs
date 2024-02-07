use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};
use codepage_437::{FromCp437, CP437_CONTROL};
use flate2::read::DeflateDecoder;

mod constants;
mod directory_central;
mod directory_end;

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
    let central_directory_end = directory_end::CentralDirectorEnd::find_and_parse(reader)
        .unwrap()
        .0;

    reader.seek(SeekFrom::Start(
        central_directory_end.central_directory_offset as u64,
    ))?;

    loop {
        if reader.read_u32::<LittleEndian>().unwrap() == 0x06054b50 {
            break;
        } else {
            reader.seek(SeekFrom::Current(-4)).unwrap();
        }

        let central_directory = directory_central::CentralDirectory::parse(reader).unwrap();

        let start_position = reader.seek(SeekFrom::Current(0)).unwrap();

        reader
            .seek(SeekFrom::Start(central_directory.relative_offset as u64))
            .unwrap();

        save_file(reader).unwrap();

        reader.seek(SeekFrom::Start(start_position)).unwrap();

        println!("{}", central_directory.file_name);
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
