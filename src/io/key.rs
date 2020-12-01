use crate::types::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Key(u16);
    pub L, _: 9;
    pub R, _: 8;
    pub DOWN, _: 7;
    pub UP, _: 6;
    pub LEFT, _: 5;
    pub RIGHT, _: 4;
    pub START, _: 3;
    pub SELECT, _: 2;
    pub B, _: 1;
    pub A, _: 0;
}

pub(crate) enum KeyStatus {
    ON,
    OFF,
}

impl Key {
    pub(crate) fn new() -> Self {
        Self(0x03FF)
    }

    pub(crate) fn set_A(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FE,
            KeyStatus::OFF => self.0 | 0x03FE,
        };
    }

    pub(crate) fn set_B(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FD,
            KeyStatus::OFF => self.0 | 0x03FD,
        };
    }

    pub(crate) fn set_SELECT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FB,
            KeyStatus::OFF => self.0 | 0x03FB,
        };
    }

    pub(crate) fn set_START(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03F7,
            KeyStatus::OFF => self.0 | 0x03F7,
        };
    }

    pub(crate) fn set_RIGHT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03E7,
            KeyStatus::OFF => self.0 | 0x03E7,
        };
    }

    pub(crate) fn set_LEFT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03DF,
            KeyStatus::OFF => self.0 | 0x03DF,
        };
    }

    pub(crate) fn set_UP(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03BF,
            KeyStatus::OFF => self.0 | 0x03BF,
        };
    }

    pub(crate) fn set_DOWN(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x037F,
            KeyStatus::OFF => self.0 | 0x037F,
        };
    }

    pub(crate) fn set_R(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x02FF,
            KeyStatus::OFF => self.0 | 0x02FF,
        };
    }

    pub(crate) fn set_L(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x01FF,
            KeyStatus::OFF => self.0 | 0x01FF,
        };
    }

    pub(crate) fn read(&self) -> HalfWord {
        self.0
    }
}
