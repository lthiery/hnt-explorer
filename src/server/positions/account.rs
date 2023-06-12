use super::*;
use cli::positions;

use crate::cli;

#[derive(Debug, Default)]
pub struct Account {
    pub balances: LockedBalances,
    pub positions: Vec<Pubkey>,
}

#[derive(Debug, Default, Copy, Clone, serde::Serialize)]
pub struct LockedBalances {
    pub vehnt: VehntBalance,
    pub locked_hnt: u64,
    pub pending_iot: u64,
    pub pending_mobile: u64,
}

impl Account {
    pub fn initialize_with_element(
        positions: &HashMap<Pubkey, positions::Position>,
        pubkey: Pubkey,
    ) -> Result<Self> {
        let mut s = Self {
            balances: LockedBalances::default(),
            positions: vec![],
        };
        s.push_entry(positions, pubkey)?;
        Ok(s)
    }

    pub fn push_entry(
        &mut self,
        positions: &HashMap<Pubkey, positions::Position>,
        pubkey: Pubkey,
    ) -> Result {
        match positions.get(&pubkey) {
            None => {
                return Err(Error::MissingPosition { position: pubkey });
            }
            Some(p) => {
                self.balances.locked_hnt += p.hnt_amount;
                self.balances.vehnt.total += p.vehnt;
                if let Some(delegated) = &p.delegated {
                    match delegated.sub_dao {
                        SubDao::Iot => {
                            self.balances.vehnt.iot_delegated += p.vehnt;
                            self.balances.pending_iot = delegated.pending_rewards;
                        }
                        SubDao::Mobile => {
                            self.balances.vehnt.mobile_delegated += p.vehnt;
                            self.balances.pending_mobile = delegated.pending_rewards;
                        }
                        SubDao::Unknown => {
                            println!(
                                "Unknown subdao for delegated position {} for account {pubkey}!",
                                p.position_key
                            );
                        }
                    }
                } else {
                    self.balances.vehnt.undelegated += p.vehnt;
                }
                self.positions.push(pubkey);
            }
        }
        Ok(())
    }
}
