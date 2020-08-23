// 31    28 27  25 24  23  22  21  20 19    16 15                      0
// ---------------------------------------------------------------------
// | cond | 1 0 0 | P | U | S | W | L |  Rn  |      Register List      |
// ---------------------------------------------------------------------
// P = 0: Post index 1: Pre index
// U = 0: Decrement 1: Increment
// S = Restore force user bit. S specifies if banked register access should occur when in privileged modes [or if R15 and 26 bit and user mode, if the PSR should be written while PC is updated]
// W = 1: Auto Index
// L = 0: Store / 1: Load
bitfield! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct BlockDataTransfer(u32);
    pub get_cond, _: 31, 28;
    #[allow(non_snake_case)]
    pub get_P, _: 24 ;
    #[allow(non_snake_case)]
    pub get_U, _: 23;
    #[allow(non_snake_case)]
    pub get_S, _: 22;
    #[allow(non_snake_case)]
    pub get_W, _: 21;
    #[allow(non_snake_case)]
    pub get_L, _: 20;
    #[allow(non_snake_case)]
    pub get_Rn, _: 19, 16;
    pub get_register_list, _: 15, 0;
}
