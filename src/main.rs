use crate::parser::{AccessRegister, Direction};
use clap::Parser;
use colored::Colorize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

mod parser;

/// Parse WaveForms SWD protocol logs
#[derive(Parser, Debug, Clone)]
#[clap(version)]
struct Opts {
    /// Input WaveForms SWD log file to read
    pub input: PathBuf,
}

// S32K344 specifics
const APB_AP_ID: u8 = 1;
const CM7_0_AHB_AP_ID: u8 = 4;
const MDM_AP_ID: u8 = 6;
const SDA_AP_ID: u8 = 7;
const S32K3XX_AP_IDS: [u8; 4] = [APB_AP_ID, CM7_0_AHB_AP_ID, MDM_AP_ID, SDA_AP_ID];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    let file = File::open(opts.input)?;

    let mut reader = BufReader::new(file);

    let mut line_buf = String::new();

    let mut observed_aps = HashSet::new();

    let mut dp_select_reg = dp_regs::Select(0);
    let mut tar_reg = ap_regs::Tar(0);

    loop {
        line_buf.clear();

        let bytes_read = reader.read_line(&mut line_buf)?;
        if bytes_read == 0 {
            break;
        }

        let line = line_buf.trim();
        print!("{}", line);

        if let Ok((_, op)) = parser::parse(line) {
            print!("  ");

            match op.direction {
                Direction::Read => print!("<-- "),
                Direction::Write => print!("{} ", "-->".yellow().bold()),
            }

            match op.access {
                AccessRegister::DebugPort => {
                    let address = op.address_2_3 << 2;
                    print!("R:{:02X}", address);

                    match address {
                        // 0x00
                        dp_regs::IdCode::ADDRESS if op.direction == Direction::Read => {
                            let idcode = dp_regs::IdCode(op.data);
                            print!(
                                " {}    Version:{} PARTNO:{} DESIGNER:{}",
                                dp_regs::IdCode::NAME,
                                idcode.version(),
                                idcode.partno(),
                                idcode.designer(),
                            );
                        }
                        dp_regs::Abort::ADDRESS if op.direction == Direction::Write => {
                            let abort = dp_regs::Abort(op.data);
                            print!(
                                " {}     DAPABORT:{} STKCMPCLR:{} STKERRCLR:{} WDERRCLR:{} ORUNERRCLR:{}",
                                dp_regs::Abort::NAME,
                                if abort.dapabort() { 1 } else { 0 },
                                if abort.stkcmpclr() { 1 } else { 0 },
                                if abort.stkerrclr() { 1 } else { 0 },
                                if abort.wderrclr() { 1 } else { 0 },
                                if abort.orunerrclr() { 1 } else { 0 },
                            );
                        }

                        // 0x04
                        dp_regs::CtrlStat::ADDRESS if !dp_select_reg.ctrlsel() => {
                            let ctrlstat = dp_regs::CtrlStat(op.data);
                            print!(
                                " {} READOK:{} WDATAERR:{} TRNCNT:{} CDBGRSTREQ:{} (ACK:{}) CDBGPWRUPREQ:{} (ACK:{}) CSYSPWRUPREQ:{} (ACK:{})",
                                dp_regs::CtrlStat::NAME,
                                if ctrlstat.readok() { 1 } else { 0 },
                                if ctrlstat.wdataerr() { 1 } else { 0 },
                                ctrlstat.trncnt(),
                                if ctrlstat.cdbgrstreq() { 1 } else { 0 },
                                if ctrlstat.cdbgrstack() { 1 } else { 0 },
                                if ctrlstat.cdbgpwrupreq() { 1 } else { 0 },
                                if ctrlstat.cdbgpwrupack() { 1 } else { 0 },
                                if ctrlstat.csyspwrupreq() { 1 } else { 0 },
                                if ctrlstat.csyspwrupack() { 1 } else { 0 },
                            );
                        }
                        dp_regs::Wcr::ADDRESS if dp_select_reg.ctrlsel() => {
                            let wcr = dp_regs::Wcr(op.data);
                            print!(
                                " {}    PRESCALER:{} WIREMODE:{} TURNROUND:{}",
                                dp_regs::Wcr::NAME,
                                wcr.prescaler(),
                                wcr.wiremode(),
                                wcr.turnround(),
                            );
                        }

                        // 0x08
                        dp_regs::Select::ADDRESS if op.direction == Direction::Write => {
                            let select = dp_regs::Select(op.data);

                            observed_aps.insert(select.apsel() as u8);

                            let mut apsel = format!("{:02X}", select.apsel()).normal();
                            if select.apsel() as u8 == MDM_AP_ID
                                || select.apsel() as u8 == SDA_AP_ID
                            {
                                apsel = apsel.bright_red();
                            } else if S32K3XX_AP_IDS.contains(&(select.apsel() as u8)) {
                                apsel = apsel.bright_yellow();
                            }

                            let s32k3xx_ap = match select.apsel() as u8 {
                                APB_AP_ID => "    (APB_AP)",
                                CM7_0_AHB_AP_ID => "    (CM7_0_AHB_AP)",
                                MDM_AP_ID => "    (MDM_AP)",
                                SDA_AP_ID => "    (SDA_API)",
                                _ => "",
                            };

                            print!(
                                " {}    APSEL:{} APBANKSEL:{:02X} CTRLSEL:{}{}",
                                dp_regs::Select::NAME,
                                //select.apsel()
                                apsel,
                                select.apbanksel(),
                                if select.ctrlsel() { 1 } else { 0 },
                                s32k3xx_ap,
                            );
                            // TODO is this right?
                            dp_select_reg = select;
                        }
                        dp_regs::Resend::ADDRESS if op.direction == Direction::Read => {
                            print!(" {}    {:08X}", dp_regs::Resend::NAME, op.data);
                        }

                        // 0x0C
                        dp_regs::RdBuff::ADDRESS if op.direction == Direction::Read => {
                            print!(" {}    {:08X}", dp_regs::RdBuff::NAME, op.data);
                        }

                        _ => {
                            panic!("Unhandled SW-DP register access");
                        }
                    }
                }
                AccessRegister::AccessPort => {
                    let address = mem_ap_address(dp_select_reg.apbanksel() as u8, op.address_2_3);

                    let is_32k3xx = dp_select_reg.apsel() as u8 == MDM_AP_ID
                        || dp_select_reg.apsel() as u8 == SDA_AP_ID;

                    if is_32k3xx {
                        print!("R:{}", format!("{:02X}", address).bright_red());
                    } else {
                        print!("R:{:02X}", address);
                    }

                    if !is_32k3xx {
                        match address {
                            ap_regs::Idr::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Idr::NAME, op.data);
                            }
                            ap_regs::Tar::ADDRESS => {
                                tar_reg.set_addr(op.data);
                                print!(" {}       {:08X}", ap_regs::Tar::NAME, op.data);
                            }
                            ap_regs::Csw::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Csw::NAME, op.data);
                            }
                            ap_regs::Drw::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Drw::NAME, op.data);

                                match tar_reg.addr() {
                                    arm_regs::Dhcsr::ADDRESS => {
                                        let dhcsr = arm_regs::Dhcsr(op.data);
                                        print!("    ");
                                        dhcsr.min_print();
                                        //print!("\n{:?}", dhcsr);
                                    }
                                    arm_regs::Demcr::ADDRESS => {
                                        let demcr = arm_regs::Demcr(op.data);
                                        print!("    ");
                                        demcr.min_print();
                                        //print!("\n{:?}", demcr);
                                    }
                                    arm_regs::Aircr::ADDRESS => {
                                        let reg = arm_regs::Aircr(op.data);
                                        print!("    ");
                                        reg.min_print();
                                        //print!("\n{:?}", reg);
                                    }
                                    _ => (),
                                }
                            }
                            ap_regs::Bd0::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Bd0::NAME, op.data);
                                match tar_reg.addr() {
                                    arm_regs::Dhcsr::ADDRESS => {
                                        let dhcsr = arm_regs::Dhcsr(op.data);
                                        print!("    ");
                                        dhcsr.min_print();
                                        //print!("\n{:?}", dhcsr);
                                    }
                                    arm_regs::Demcr::ADDRESS => {
                                        let demcr = arm_regs::Demcr(op.data);
                                        print!("    ");
                                        demcr.min_print();
                                        //print!("\n{:?}", demcr);
                                    }
                                    arm_regs::Aircr::ADDRESS => {
                                        let reg = arm_regs::Aircr(op.data);
                                        print!("    ");
                                        reg.min_print();
                                        //print!("\n{:?}", reg);
                                    }
                                    _ => (),
                                }
                            }
                            ap_regs::Bd1::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Bd1::NAME, op.data);
                            }
                            ap_regs::Bd2::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Bd2::NAME, op.data);
                            }
                            ap_regs::Bd3::ADDRESS => {
                                print!(" {}       {:08X}", ap_regs::Bd3::NAME, op.data);
                            }
                            _ => {
                                print!("           {:08X}", op.data);
                            }
                        }
                    } else {
                        print!("           {:08X}", op.data);
                        // is S32K344
                        if dp_select_reg.apsel() as u8 == SDA_AP_ID {
                            match address {
                                0x80 => print!(
                                    "                           ({})",
                                    "DBGENCTRL".bright_red()
                                ),
                                0x90 => print!(
                                    "                           ({})",
                                    "SDAAPRSTCTRL".bright_red()
                                ),
                                0xFC => {
                                    print!("                           ({})", "ID".bright_red())
                                }
                                _ => print!("                           ({})", "TODO add reg"),
                            }
                        }
                    }
                }
            }
        }

        println!();
    }

    println!("---------------------------------------------------");
    println!("Observed APs:");
    for ap in observed_aps.into_iter() {
        println!("  {} (0x:{:02X})", ap, ap);
    }

    Ok(())
}

fn mem_ap_address(apbanksel: u8, address_2_3: u8) -> u8 {
    (apbanksel << 4) | (address_2_3 << 2)
}

mod dp_regs {
    use bitfield::bitfield;

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct IdCode(u32);
        pub designer, _ : 11, 1;
        pub partno, _ : 27, 12;
        pub version, _ : 31, 28;
    }

    impl IdCode {
        pub const ADDRESS: u8 = 0x00;
        pub const NAME: &'static str = "IDCODE";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Abort(u32);
        pub dapabort, _ : 0;
        pub stkcmpclr, _ : 1;
        pub stkerrclr, _ : 2;
        pub wderrclr, _ : 3;
        pub orunerrclr, _ : 4;
    }

    impl Abort {
        pub const ADDRESS: u8 = 0x00;
        pub const NAME: &'static str = "ABORT";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct CtrlStat(u32);
        pub readok, _ : 6;
        pub wdataerr, _ : 7;
        pub trncnt, _ : 21, 12;
        /// Debug reset request
        pub cdbgrstreq, _ : 26;
        /// Debug reset acknowledge
        pub cdbgrstack, _ : 27;
        /// Debug power-up request
        pub cdbgpwrupreq, _ : 28;
        /// Debug power-up acknowledge
        pub cdbgpwrupack, _ : 29;
        /// System power-up request
        pub csyspwrupreq, _ : 30;
        /// System power-up acknowledge
        pub csyspwrupack, _ : 31;
    }

    impl CtrlStat {
        pub const ADDRESS: u8 = 0x04;
        pub const NAME: &'static str = "CTRL/STAT";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Wcr(u32);
        pub prescaler, _ : 2, 0;
        pub wiremode, _ : 7, 6;
        pub turnround, _ : 9, 8;
    }

    impl Wcr {
        pub const ADDRESS: u8 = 0x04;
        pub const NAME: &'static str = "WCR";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Select(u32);
        pub ctrlsel, _ : 0;
        pub apbanksel, _ : 7, 4;
        pub apsel, _ : 31, 24;
    }

    impl Select {
        pub const ADDRESS: u8 = 0x08;
        pub const NAME: &'static str = "SELECT";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Resend(u32);
    }

    impl Resend {
        pub const ADDRESS: u8 = 0x08;
        pub const NAME: &'static str = "RESEND";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct RdBuff(u32);
    }

    impl RdBuff {
        pub const ADDRESS: u8 = 0x0C;
        pub const NAME: &'static str = "RDBUFF";
    }
}

mod ap_regs {
    use bitfield::bitfield;

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Csw(u32);
    }

    impl Csw {
        pub const ADDRESS: u8 = 0x00;
        pub const NAME: &'static str = "CSW";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Tar(u32);
        pub addr, set_addr : 31, 0;
    }

    impl Tar {
        pub const ADDRESS: u8 = 0x04;
        pub const NAME: &'static str = "TAR";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Drw(u32);
    }

    impl Drw {
        pub const ADDRESS: u8 = 0x0C;
        pub const NAME: &'static str = "DRW";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Bd0(u32);
    }

    impl Bd0 {
        pub const ADDRESS: u8 = 0x10;
        pub const NAME: &'static str = "BD0";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Bd1(u32);
    }

    impl Bd1 {
        pub const ADDRESS: u8 = 0x14;
        pub const NAME: &'static str = "BD1";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Bd2(u32);
    }

    impl Bd2 {
        pub const ADDRESS: u8 = 0x18;
        pub const NAME: &'static str = "BD2";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Bd3(u32);
    }

    impl Bd3 {
        pub const ADDRESS: u8 = 0x1C;
        pub const NAME: &'static str = "BD3";
    }

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct Idr(u32);
    }

    impl Idr {
        pub const ADDRESS: u8 = 0xFC;
        pub const NAME: &'static str = "IDR";
    }
}

mod arm_regs {
    use bitfield::bitfield;
    use colored::Colorize;

    bitfield! {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct Dhcsr(u32);
        impl Debug;
        pub s_reset_st, _: 25;
        pub s_retire_st, _: 24;
        pub s_lockup, _: 19;
        pub s_sleep, _: 18;
        pub s_halt, _: 17;
        pub s_regrdy, _: 16;
        pub c_maskints, set_c_maskints: 3;
        pub c_step, set_c_step: 2;
        pub c_halt, set_c_halt: 1;
        pub c_debugen, set_c_debugen: 0;
    }

    impl Dhcsr {
        pub const ADDRESS: u32 = 0xE000_EDF0;
        pub const NAME: &'static str = "DHCSR";
    }

    impl Dhcsr {
        pub fn min_print(&self) {
            print!(
                "{} (s_reset_st:{}, s_halt:{}, c_halt:{}, c_debugen:{})",
                Self::NAME.bright_blue(),
                self.s_reset_st() as u8,
                self.s_halt() as u8,
                self.c_halt() as u8,
                self.c_debugen() as u8,
            );
        }
    }

    bitfield! {
        /// Debug Exception and Monitor Control Register, DEMCR (see armv7-M Architecture Reference Manual C1.6.5)
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct Demcr(u32);
        impl Debug;
        /// Global enable for DWT and ITM features
        pub trcena, set_trcena: 24;
        /// DebugMonitor semaphore bit
        pub mon_req, set_mon_req: 19;
        /// Step the processor?
        pub mon_step, set_mon_step: 18;
        /// Sets or clears the pending state of the DebugMonitor exception
        pub mon_pend, set_mon_pend: 17;
        /// Enable the DebugMonitor exception
        pub mon_en, set_mon_en: 16;
        /// Enable halting debug trap on a HardFault exception
        pub vc_harderr, set_vc_harderr: 10;
        /// Enable halting debug trap on a fault occurring during exception entry
        /// or exception return
        pub vc_interr, set_vc_interr: 9;
        /// Enable halting debug trap on a BusFault exception
        pub vc_buserr, set_vc_buserr: 8;
        /// Enable halting debug trap on a UsageFault exception caused by a state
        /// information error, for example an Undefined Instruction exception
        pub vc_staterr, set_vc_staterr: 7;
        /// Enable halting debug trap on a UsageFault exception caused by a
        /// checking error, for example an alignment check error
        pub vc_chkerr, set_vc_chkerr: 6;
        /// Enable halting debug trap on a UsageFault caused by an access to a
        /// Coprocessor
        pub vc_nocperr, set_vc_nocperr: 5;
        /// Enable halting debug trap on a MemManage exception.
        pub vc_mmerr, set_vc_mmerr: 4;
        /// Enable Reset Vector Catch
        pub vc_corereset, set_vc_corereset: 0;
    }

    impl Demcr {
        pub const ADDRESS: u32 = 0xE000_EDFC;
        pub const NAME: &'static str = "DEMCR";
    }

    impl Demcr {
        pub fn min_print(&self) {
            print!(
                "{} (trcena:{}, vc_harderr:{}, vc_corereset:{})",
                Self::NAME.bright_blue(),
                self.trcena() as u8,
                self.vc_harderr() as u8,
                self.vc_corereset() as u8,
            );
        }
    }

    bitfield! {
        /// Application Interrupt and Reset Control Register, AIRCR (see armv7-M Architecture Reference Manual B3.2.6)
        ///
        /// [`Aircr::vectkey`] must be called before this register can effectively be written!
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct Aircr(u32);
        impl Debug;
        /// Vector Key. The value 0x05FA must be written to this register, otherwise
        /// the register write is UNPREDICTABLE.
        get_vectkeystat, set_vectkey: 31,16;
        /// Indicates the memory system data endianness:
        ///
        /// `0`: little endian.\
        /// `1`: big endian.
        ///
        /// See Endian support on page A3-44 for more information.
        pub endianness, set_endianness: 15;
        /// Priority grouping, indicates the binary point position.
        ///
        /// For information about the use of this field see Priority grouping on page B1-527.
        ///
        /// This field resets to `0b000`.
        pub prigroup, set_prigroup: 10,8;
        /// System Reset Request:
        ///
        /// `0`: do not request a reset.\
        /// `1`: request reset.
        ///
        /// Writing `1` to this bit asserts a signal to request a reset by the external
        /// system. The system components that are reset by this request are
        /// IMPLEMENTATION DEFINED. A Local reset is required as part of a system
        /// reset request.
        ///
        /// A Local reset clears this bit to `0`.
        ///
        /// See Reset management on page B1-208 for more information
        pub sysresetreq, set_sysresetreq: 2;
        /// Clears all active state information for fixed and configurable exceptions:
        ///
        /// `0`: do not clear state information.\
        /// `1`: clear state information.
        ///
        /// The effect of writing a `1` to this bit if the processor is not halted in Debug
        /// state is UNPREDICTABLE.
        pub vectclractive, set_vectclractive: 1;
        /// Writing `1` to this bit causes a local system reset, see Reset management on page B1-559 for
        /// more information. This bit self-clears.
        ///
        /// The effect of writing a `1` to this bit if the processor is not halted in Debug state is UNPREDICTABLE.
        ///
        /// When the processor is halted in Debug state, if a write to the register writes a `1` to both
        /// VECTRESET and SYSRESETREQ, the behavior is UNPREDICTABLE.
        ///
        /// This bit is write only.
        pub vectreset, set_vectreset: 0;
    }

    impl Aircr {
        pub const ADDRESS: u32 = 0xE000_ED0C;
        pub const NAME: &'static str = "AIRCR";
    }

    impl Aircr {
        pub fn min_print(&self) {
            print!(
                "{} (sysresetreq:{}, vectclractive:{}, vectreset:{})",
                Self::NAME.bright_blue(),
                self.sysresetreq() as u8,
                self.vectclractive() as u8,
                self.vectreset() as u8,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_ap_address_sanity() {
        assert_eq!(mem_ap_address(0x0, 0b00), 0x00);
        assert_eq!(mem_ap_address(0x0, 0b01), 0x04);
        assert_eq!(mem_ap_address(0x0, 0b11), 0x0C);
        assert_eq!(mem_ap_address(0x1, 0b10), 0x18);
        assert_eq!(mem_ap_address(0xF, 0b10), 0xF8);
        assert_eq!(mem_ap_address(0xF, 0b11), 0xFC);
    }
}
