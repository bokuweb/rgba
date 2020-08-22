// use super::super::PipelineStatus;
//
// use crate::cpu::bus::accessor::*;
// use crate::cpu::constants::*;
// use crate::cpu::decoder::arm::*;
// use crate::cpu::registers::psr::PSR;
// use crate::cpu::types::*;
//
// pub fn exec_msr<T>(
//     _bus: &mut T,
//     dec: Box<dyn Decoder>,
//     gpr: &mut [Word; 16],
//     _cspr: &PSR,
// ) -> Result<PipelineStatus, ()>
// where
//     T: BusAccessor,
// {
//     let rd = dec.get_Rd();
//     let rn = dec.get_Rn();
//     let rm = dec.get_Rm();
//     let ra = dec.get_Ra();
//     exec_multiple(gpr, dec, &mut |gpr| {
//         let mul = (gpr[rn] as u64) * gpr[rm] as u64;
//         let base = ((gpr[rd] as u64) << 32) + (gpr[ra] as u64);
//         let result = mul + base;
//         gpr[ra] = result as u32;
//         gpr[rd] = (result >> 32) as u32;
//     })
// }
//
