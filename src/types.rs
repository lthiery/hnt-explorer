use super::Result;
use anchor_lang::solana_program::pubkey::Pubkey;

pub const HELIUM_DAO_ID: &str = "hdaoVTCqhfHHo75XdAMxBKdUqvq1i5bF23sisBqVgGR";
pub const HELIUM_VSR_ID: &str = "hvsrNC3NKbcryqDs2DocYHZ9yPKEVzdSjQG6RVtK1s8";

pub const HNT_MINT: &str = "hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux";
pub const MOBILE_MINT: &str = "mb1eu7TzEc71KxDpsmsKoucSSuuoGLv1drys1oP2jh6";
pub const IOT_MINT: &str = "iotEVVZLEywoTn1QdwNPddxPWszn3zFhEot3MfL9fns";

pub const IOT_SUBDAO: &str = "39Lw1RH6zt8AJvKn3BTxmUDofzduCM2J3kSaGDZ8L7Sk";
pub const MOBILE_SUBDAO: &str = "Gm9xDCJawDEKDrrQW6haw94gABaYzQwCq4ZQU8h8bd22";

pub const TOKEN_DIVIDER: u128 = 100_000_000; // 10^8
pub const DNT_DIVIDER: u128 = 1_000_000; // 10^6

pub const ANOTHER_DIVIDER: u128 =
    TOKEN_DIVIDER * helium_anchor_gen::voter_stake_registry::PRECISION_FACTOR;

#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubDao {
    #[default]
    Unknown,
    Iot,
    Mobile,
}

impl TryFrom<Pubkey> for SubDao {
    type Error = super::error::Error;
    fn try_from(pubkey: Pubkey) -> Result<Self> {
        match pubkey.to_string().as_str() {
            IOT_SUBDAO => Ok(SubDao::Iot),
            MOBILE_SUBDAO => Ok(SubDao::Mobile),
            _ => Err(Self::Error::InvalidSubDao(pubkey)),
        }
    }
}
