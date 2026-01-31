pub trait UsernameString {
    fn normalize(&self) -> String;
}

impl UsernameString for &str {
    fn normalize(&self) -> String {
        self.to_lowercase().replace(' ', "_")
    }
}

impl UsernameString for String {
    fn normalize(&self) -> String {
        self.as_str().normalize()
    }
}
