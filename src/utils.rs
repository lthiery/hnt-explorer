use super::types::*;

pub fn format_dnt(dnt: u64) -> String {
    let dnt = dnt / DNT_DIVIDER as u64;
    dnt.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

pub fn format_hnt(vehnt: u64) -> String {
    let vehnt = vehnt / TOKEN_DIVIDER as u64;
    vehnt
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

pub fn format_vehnt(vehnt: u128) -> String {
    let vehnt = vehnt / ANOTHER_DIVIDER;
    vehnt
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}
