bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct DataProcessing(u16);
    pub get_op, _: 12, 11;
    pub get_sh, _: 10, 6;
    #[allow(non_snake_case)]
    pub get_Rn, _: 5, 3;
    #[allow(non_snake_case)]
    pub get_Rd, _: 3, 0;
}

