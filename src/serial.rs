
pub fn write_float<W>(f: &mut W, value: f32, precision: u8) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
{
    let (number, decimals) = float_to_int_f32(value, precision);
    ufmt::uwrite!(f, "{}", number)?;
    match precision {
        1 => ufmt::uwrite!(f, ".{}", decimals),
        2 if decimals >= 10 => ufmt::uwrite!(f, ".{}", decimals),
        2 if decimals < 10 => ufmt::uwrite!(f, ".0{}", decimals),
        3 if decimals >= 100 => ufmt::uwrite!(f, ".{}", decimals),
        3 if decimals < 100 => ufmt::uwrite!(f, ".0{}", decimals),
        3 if decimals < 10 => ufmt::uwrite!(f, ".00{}", decimals),
        4 if decimals >= 1000 => ufmt::uwrite!(f, ".{}", decimals),
        4 if decimals < 1000 && decimals >= 100 => ufmt::uwrite!(f, ".0{}", decimals),
        4 if decimals < 100 && decimals >= 10 => ufmt::uwrite!(f, ".00{}", decimals),
        4 if decimals < 10 => ufmt::uwrite!(f, ".000{}", decimals),
        5 if decimals >= 10000 => ufmt::uwrite!(f, ".{}", decimals),
        5 if decimals < 10000 && decimals >= 1000 => ufmt::uwrite!(f, ".0{}", decimals),
        5 if decimals < 1000 && decimals >= 100 => ufmt::uwrite!(f, ".00{}", decimals),
        5 if decimals < 100 && decimals >= 10 => ufmt::uwrite!(f, ".000{}", decimals),
        5 if decimals < 10 => ufmt::uwrite!(f, ".0000{}", decimals),
        _ => Ok(()),
    }
}

///Split the float into the integer and the fraction with the correct precision
fn float_to_int_f32(original: f32,precision: u8) -> (i32,u32) {
    let prec = match precision {
        1 => 10.0,
        2 => 100.0,
        3 => 1000.0,
        4 => 10000.0,
        5 => 100000.0,
        _ => 0.0,
    };
    let base = original as i32;
    let decimal = ((original - (base as f32)) * prec) as u32;
    (base,decimal)
}
