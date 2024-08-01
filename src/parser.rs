use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, hex_digit1, space0},
    combinator::{map, map_res, value},
    sequence::{preceded, tuple},
    IResult,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SwdOperation {
    pub access: AccessRegister,
    pub direction: Direction,
    pub address_2_3: u8,
    pub ack: u8,
    pub data: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AccessRegister {
    DebugPort,
    AccessPort,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Direction {
    Write,
    Read,
}

pub fn parse(s: &str) -> IResult<&str, SwdOperation> {
    map(
        tuple((
            preceded(space0, access),
            preceded(space0, direction),
            preceded(space0, address_2_3),
            preceded(space0, ack),
            preceded(space0, ok_respose_data),
        )),
        |(access, direction, address_2_3, ack, data)| SwdOperation {
            access,
            direction,
            address_2_3,
            ack,
            data,
        },
    )(s)
}

fn access(s: &str) -> IResult<&str, AccessRegister> {
    alt((
        value(AccessRegister::AccessPort, tag("AP")),
        value(AccessRegister::DebugPort, tag("DP")),
    ))(s)
}

fn direction(s: &str) -> IResult<&str, Direction> {
    alt((
        value(Direction::Write, tag("WR")),
        value(Direction::Read, tag("RD")),
    ))(s)
}

fn address_2_3(s: &str) -> IResult<&str, u8> {
    preceded(tag("A:"), map_res(digit1, str::parse))(s)
}

fn ack(s: &str) -> IResult<&str, u8> {
    preceded(tag("ACK:"), map_res(digit1, str::parse))(s)
}

fn ok_respose_data(s: &str) -> IResult<&str, u32> {
    preceded(
        tag("OK Data:h"),
        map_res(hex_digit1, |out: &str| u32::from_str_radix(out, 16)),
    )(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_access() {
        assert_eq!(access("AP"), Ok(("", AccessRegister::AccessPort)));
        assert_eq!(access("DP"), Ok(("", AccessRegister::DebugPort)));
    }

    #[test]
    fn parse_direction() {
        assert_eq!(direction("WR"), Ok(("", Direction::Write)));
        assert_eq!(direction("RD"), Ok(("", Direction::Read)));
    }

    #[test]
    fn parse_address_2_3() {
        assert_eq!(address_2_3("A:0"), Ok(("", 0)));
        assert_eq!(address_2_3("A:1"), Ok(("", 1)));
        assert_eq!(address_2_3("A:2"), Ok(("", 2)));
        assert_eq!(address_2_3("A:3"), Ok(("", 3)));
    }

    #[test]
    fn parse_ack() {
        assert_eq!(ack("ACK:0"), Ok(("", 0)));
        assert_eq!(ack("ACK:1"), Ok(("", 1)));
        assert_eq!(ack("ACK:2"), Ok(("", 2)));
    }

    #[test]
    fn parse_response_data() {
        assert_eq!(ok_respose_data("OK Data:h00030003"), Ok(("", 0x0003_0003)));
    }

    #[test]
    fn parse_op() {
        assert_eq!(
            parse("AP RD A:1 ACK:1 OK Data:h61000003"),
            Ok((
                "",
                SwdOperation {
                    access: AccessRegister::AccessPort,
                    direction: Direction::Read,
                    address_2_3: 1,
                    ack: 1,
                    data: 0x61000003,
                }
            ))
        );
    }
}
