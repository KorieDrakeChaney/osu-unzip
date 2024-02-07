mod read;

pub use read::{get_all_osu_maps, unzip_osz};

#[cfg(test)]
mod tests {

    use std::io::Read;

    use super::*;

    #[test]
    fn test() {
        let files = read::unzip_osz("test.osz").unwrap();

        for (k, v) in files {
            if k.ends_with(".osu") {
                let mut reader = std::fs::File::open(v).unwrap();
                let mut osu_string = String::new();
                reader.read_to_string(&mut osu_string).unwrap();
                println!("{}", osu_string);
            }
        }
    }

    #[test]
    fn test2() {
        let files = read::get_all_osu_maps().unwrap();

        for (_, v) in files {
            println!("{}", v.to_str().unwrap());
        }
    }
}
