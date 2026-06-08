bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DataProcessing(u16);
    pub get_op12_11, _: 12, 11;
    pub get_op10_9, _: 9, 6;
    pub get_op9_8, _: 9, 8;
    pub get_op9_6, _: 9, 6;
    pub get_bit11, _: 11;
    #[allow(non_snake_case)]
    pub get_Rd10_8, _: 10, 8;
    #[allow(non_snake_case)]
    pub get_Rn10_8, _: 10, 8;
    pub get_sh, _: 10, 6;
    pub get_imm3, _: 8, 6;
    #[allow(non_snake_case)]
    pub get_Rm8_6, _: 8, 6;
    #[allow(non_snake_case)]
    pub get_msbd, _: 7;
    #[allow(non_snake_case)]
    pub get_Rn5_3, _: 5, 3;
    #[allow(non_snake_case)]
    pub get_Rs6_3, _: 6, 3;
    #[allow(non_snake_case)]
    pub get_Rs, _: 5, 3;
    #[allow(non_snake_case)]
    pub get_Rm5_3, _: 5, 3;
    #[allow(non_snake_case)]
    pub get_A, _: 7;
    pub get_imm7, _: 6, 0;
    pub get_imm8, _: 7, 0;
    #[allow(non_snake_case)]
    pub get_Rd2_0, _: 2, 0;
}

impl DataProcessing {
    // `Rd` is the THUMB destination-register field; the name matches the encoding.
    #[allow(non_snake_case)]
    pub fn get_Rd_7_2_0(&self) -> u16 {
        let rd = self.get_Rd2_0();
        if self.get_msbd() {
            rd + 0x8
        } else {
            rd
        }
    }
}
