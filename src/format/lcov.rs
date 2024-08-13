#[cfg(test)]
mod test {
    use crate::format::path_normalize;
    use std::env;

    #[test]
    fn test_lcov() {
        use lcov2::Records;
        use std::str::FromStr;

        let contents = include_str!("../../tmp/coverage/lcov.info");
        let records = Records::from_str(&contents).unwrap();


        let mut cwd = env::current_dir().unwrap();
        cwd.push("tmp");
        env::set_current_dir(&cwd).unwrap();
        let dir = dbg!(path_normalize(cwd.to_str().unwrap()));
        records.to_html(dir).unwrap();
    }
}
