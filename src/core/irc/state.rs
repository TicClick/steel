#[derive(Debug)]
pub struct IRCState {
    pub own_nickname: Option<String>,
}

impl IRCState {
    pub fn new() -> Self {
        Self { own_nickname: None }
    }

    pub fn set_own_nickname(&mut self, nickname: String) {
        self.own_nickname = Some(nickname);
    }

    pub fn clear(&mut self) {
        self.own_nickname = None;
    }
}

impl Default for IRCState {
    fn default() -> Self {
        Self::new()
    }
}
