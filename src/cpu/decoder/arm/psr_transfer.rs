bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct PsrTransfer(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_I, _: 25 ;
    #[allow(non_snake_case)]
    pub get_Pd, _: 22;
    #[allow(non_snake_case)]
    pub get_F, _: 19;
    #[allow(non_snake_case)]
    pub get_S, _: 18;
    #[allow(non_snake_case)]
    pub get_X, _: 17;
    #[allow(non_snake_case)]
    pub get_C, _: 16;
    #[allow(non_snake_case)]
    pub get_Rd, _: 15, 12;
    pub get_rotate, _: 11, 8;
    pub get_imm, _: 7, 0;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}
