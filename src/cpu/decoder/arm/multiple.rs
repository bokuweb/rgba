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
    pub get_RdHi, _: 19, 16;
    #[allow(non_snake_case)]
    pub get_RdLo, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_Rn, _: 15, 12;
    #[allow(non_snake_case)]
    pub get_Rs, _: 11, 8;
    #[allow(non_snake_case)]
    pub get_Rm, _: 3, 0;
}
