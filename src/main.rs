use crate::parser::{AccessRegister, Direction, SwdOperation};
use clap::Parser;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    let file = File::open(opts.input)?;

    let mut reader = BufReader::new(file);

    let mut line_buf = String::new();

    let mut dp_select_reg = dp_regs::Select(0);

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
                Direction::Write => print!("--> "),
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
                            print!(
                                " {}    APSEL:{:02X} APBANKSEL:{:02X} CTRLSEL:{}",
                                dp_regs::Select::NAME,
                                select.apsel(),
                                select.apbanksel(),
                                if select.ctrlsel() { 1 } else { 0 },
                            );
                            // TODO is this right?
                            dp_select_reg = select;
                        }
                        dp_regs::Resend::ADDRESS if op.direction == Direction::Read => {
                            print!(" {}    DATA:{:X}", dp_regs::Resend::NAME, op.data);
                        }

                        // 0x0C
                        dp_regs::RdBuff::ADDRESS if op.direction == Direction::Read => {
                            print!(" {}    DATA:{:X}", dp_regs::RdBuff::NAME, op.data);
                        }

                        _ => {
                            panic!("Unhandled SW-DP register access");
                        }
                    }
                }
                AccessRegister::AccessPort => {
                    let address = mem_ap_address(dp_select_reg.apbanksel() as u8, op.address_2_3);
                    print!("R:{:02X}", address);
                }
            }
        }

        println!();
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
