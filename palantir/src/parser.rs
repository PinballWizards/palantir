use crate::transport::DataFrame;
use nom::{
    multi::length_data,
    number::streaming::{le_u16, le_u8},
    sequence::tuple,
    IResult,
};
fn parse_address(i: &[u8]) -> IResult<&[u8], u8> {
    let (input, val) = le_u8(i)?;
    Ok((input, val >> 4))
}

fn parse_datalength(i: &[u8]) -> IResult<&[u8], u8> {
    le_u8(i)
}

fn parse_app_data(i: &[u8]) -> IResult<&[u8], &[u8]> {
    length_data(parse_datalength)(i)
}

fn parse_crc(i: &[u8]) -> IResult<&[u8], u16> {
    le_u16(i)
}

/// Parses a complete data frame from a u8 slice.
pub fn parse_dataframe(i: &[u8]) -> IResult<&[u8], DataFrame> {
    let (input, (addr, data, crcval)) = tuple((parse_address, parse_app_data, parse_crc))(i)?;
    Ok((
        input,
        DataFrame::new_from_raw(addr, data.iter().cloned().collect(), crcval),
    ))
}

pub fn parse_dataframe_noclone(i: &[u8]) -> IResult<&[u8], (u8, &[u8], u16)> {
    let (input, (addr, data, crcval)) = tuple((parse_address, parse_app_data, parse_crc))(i)?;
    Ok((input, (addr, data, crcval)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_test() {
        let data = [0x10u8];
        let address = parse_address(&data);

        match address {
            Ok((_, o)) => {
                println!("parsed address: {}", o);
            }
            _ => {
                println!("failed to parse address!");
                panic!("cannot parse address");
            }
        }
    }

    #[test]
    fn parse_crc_check() {
        let val = 0x670fu16;
        match parse_crc(&val.to_le_bytes()) {
            Ok((_, o)) => {
                assert_eq!(o, val);
            }
            _ => (),
        }
    }
}
