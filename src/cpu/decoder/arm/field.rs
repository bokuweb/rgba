bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct DataProcessing(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_I, _: 25;
    pub get_opcode, _: 24, 21;
    #[allow(non_snake_case)]
    pub get_S, _: 20;
    #[allow(non_snake_case)]
    pub get_Rn, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_Rd, _: 15, 12;
    pub get_shamt5, _: 11, 7;
    pub get_sh, _: 6, 5;
    #[allow(non_snake_case)]
    pub get_bit4, _: 4;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
    #[allow(non_snake_case)]
    pub get_Rs, _: 11, 8;
    pub get_rotate, _: 11, 8;
    pub get_imm, _: 7, 0;
}

bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct Multiple(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_U, _: 22 ;
    #[allow(non_snake_case)]
    pub get_A, _: 21;
    #[allow(non_snake_case)]
    pub get_S, _: 20;
    #[allow(non_snake_case)]
    pub get_Rd, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_Ra, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_Rm, _: 11, 8;
    #[allow(non_snake_case)]
    pub get_Rn, _: 3, 0;
}

bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct Branch(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_L, _: 24;
    pub get_offset, _: 23, 0;
}
