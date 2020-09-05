bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct DataProcessing(u16);
    pub get_op, _: 12, 11;
    #[allow(non_snake_case)]
    pub get_Rd10_8, _: 10, 8;
    pub get_sh, _: 10, 6;
    pub get_imm3, _: 8, 6;
    #[allow(non_snake_case)]
    pub get_Rm, _: 8, 6;
    #[allow(non_snake_case)]
    pub get_Rn, _: 5, 3;
    pub get_imm8, _: 7, 0;
    #[allow(non_snake_case)]
    pub get_Rd2_0, _: 2, 0;
}
