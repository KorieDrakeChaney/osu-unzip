mod constants;
mod read;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        println!("{:?}", read::parse_osz("./test.osz").unwrap());
    }
}
