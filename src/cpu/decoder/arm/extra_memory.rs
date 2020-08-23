bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct ExtraMemory(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_P, _: 24 ;
    #[allow(non_snake_case)]
    pub get_U, _: 23;
    #[allow(non_snake_case)]
    pub get_I, _: 22;
    #[allow(non_snake_case)]
    pub get_W, _: 21;
    #[allow(non_snake_case)]
    pub get_L, _: 20;
    #[allow(non_snake_case)]
    pub get_Rn, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_Rd, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_imm7_4, _: 11, 8;
    #[allow(non_snake_case)]
    pub get_op2, _: 6, 5;
    #[allow(non_snake_case)]
    pub get_imm3_0, _: 3, 0;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}

impl ExtraMemory {
    pub fn get_imm8(&self) -> u32 {
        (self.get_imm7_4()).wrapping_shl(4) + (self.get_imm3_0())
    }
}
