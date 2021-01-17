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

impl Default for Key {
    fn default() -> Self {
        Self(0x03FF)
    }
}

impl Key {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn set_A(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FE,
            KeyStatus::OFF => self.0 | 0x0001,
        };
    }

    pub(crate) fn set_B(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FD,
            KeyStatus::OFF => self.0 | 0x0002,
        };
    }

    pub(crate) fn set_SELECT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03FB,
            KeyStatus::OFF => self.0 | 0x0004,
        };
    }

    pub(crate) fn set_START(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03F7,
            KeyStatus::OFF => self.0 | 0x0008,
        };
    }

    pub(crate) fn set_RIGHT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03EF,
            KeyStatus::OFF => self.0 | 0x0010,
        };
    }

    pub(crate) fn set_LEFT(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03DF,
            KeyStatus::OFF => self.0 | 0x0020,
        };
    }

    pub(crate) fn set_UP(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x03BF,
            KeyStatus::OFF => self.0 | 0x0040,
        };
    }

    pub(crate) fn set_DOWN(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x037F,
            KeyStatus::OFF => self.0 | 0x0080,
        };
    }

    pub(crate) fn set_R(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x02FF,
            KeyStatus::OFF => self.0 | 0x0100,
        };
    }

    pub(crate) fn set_L(&mut self, s: KeyStatus) {
        self.0 = match s {
            KeyStatus::ON => self.0 & 0x01FF,
            KeyStatus::OFF => self.0 | 0x0200,
        };
    }

    pub(crate) fn read(&self) -> HalfWord {
        dbg!("----", self.0);
        self.0
    }
}
