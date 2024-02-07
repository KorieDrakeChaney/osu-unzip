use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{Read, Seek, SeekFrom, Write},
    path::{self, PathBuf},
};

use byteorder::{LittleEndian, ReadBytesExt};
use codepage_437::{FromCp437, CP437_CONTROL};
use flate2::read::DeflateDecoder;
use walkdir::WalkDir;

const CENTRAL_DIRECTOR_BEGIN_SIGNATURE: u32 = 0x02014b50;
const CENTRAL_DIRECTOR_END_SIGNATURE: u32 = 0x06054b50;
const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x04034b50;

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

fn save_file<T: Read + Seek>(reader: &mut T, dir: PathBuf) -> std::io::Result<OsString> {
    std::fs::create_dir_all(dir.clone())?;

    if reader.read_u32::<LittleEndian>()? != LOCAL_FILE_HEADER_SIGNATURE {
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

    let dir = dir.join(String::from_cp437(file_name, &CP437_CONTROL));

    let file_name = dir.as_os_str();

    let mut file = std::fs::File::create(file_name)?;

    file.write_all(&decompressed_data)?;
    Ok(file_name.to_os_string())
}

fn get_local_headers<T: Read + Seek>(reader: &mut T) -> std::io::Result<HashMap<String, u64>> {
    let mut headers = HashMap::new();

    let central_directory_end = find_directory_end(reader)?;

    reader.seek(SeekFrom::Start(central_directory_end))?;

    reader.seek(SeekFrom::Current(16))?;

    let central_directory_offset = reader.read_u32::<LittleEndian>()?;

    reader.seek(SeekFrom::Start(central_directory_offset as u64))?;

    loop {
        if reader.read_u32::<LittleEndian>()? == 0x06054b50 {
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

        let relative_offset = reader.read_u32::<LittleEndian>()?;

        let mut file_name_bytes = vec![0; file_name_length as usize];

        reader.read_exact(&mut file_name_bytes)?;

        let file_name = String::from_cp437(file_name_bytes, &CP437_CONTROL);

        headers.insert(file_name, relative_offset as u64);

        reader.seek(SeekFrom::Current(
            (0 + file_comment_length + extra_field_length) as i64,
        ))?;
    }

    Ok(headers)
}

pub fn unzip_osz(file: &str) -> std::io::Result<HashMap<String, OsString>> {
    let path = path::Path::new(file);
    if path.extension() != Some(OsStr::new("osz")) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid file extension",
        ));
    } else {
        let mut files = HashMap::new();
        let path = path::Path::new(file);
        let file = path.file_name().unwrap().to_str().unwrap();
        let reader = &mut std::fs::File::open(file)?;

        let file = &file[0..file.len() - 4];
        let headers = get_local_headers(reader)?;

        if let Some(dir) = dirs::data_local_dir() {
            let dir = dir.join("osu!").join("Songs").join(file);
            for (key, value) in headers.iter() {
                reader.seek(SeekFrom::Start(*value))?;

                if let Ok(file_name) = save_file(reader, dir.clone()) {
                    files.insert(key.clone(), file_name);
                }
            }
        }

        Ok(files)
    }
}

pub fn get_all_osu_maps() -> std::io::Result<HashMap<String, OsString>> {
    let mut maps = HashMap::new();

    let dir = dirs::data_local_dir().unwrap().join("osu!").join("Songs");

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if entry.path().extension() == Some(OsStr::new("osu")) {
                let file_name = entry.file_name().to_str().unwrap().to_string();
                maps.insert(
                    file_name,
                    entry.path().to_path_buf().as_os_str().to_os_string(),
                );
            }
        }
    }

    Ok(maps)
}
