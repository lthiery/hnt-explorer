use super::*;

#[derive(Debug, Default)]
pub struct Account {
    pub balances: LockedBalances,
    pub positions: Positions,
}

#[derive(Debug, Default)]
pub struct Positions {
    pub vehnt: Vec<Pubkey>,
    pub veiot: Vec<Pubkey>,
    pub vemobile: Vec<Pubkey>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Dao {
    Hnt,
    Iot,
    Mobile,
}

#[derive(Debug, Default, Copy, Clone, serde::Serialize)]
pub struct LockedBalances {
    pub vehnt: VehntBalance,
    pub locked_hnt: u64,
    // pending from vehnt delegation
    pub pending_iot: u64,
    pub pending_mobile: u64,

    // subdao locks and voting shares
    pub veiot: u128,
    pub locked_iot: u64,
    pub vemobile: u128,
    pub locked_mobile: u64,
}

impl Account {
    pub fn initialize_with_element(
        dao: Dao,
        positions: &HashMap<Pubkey, Position>,
        pubkey: Pubkey,
    ) -> Result<Self> {
        let mut s = Self {
            balances: LockedBalances::default(),
            positions: Positions::default(),
        };
        s.push_entry(dao, positions, pubkey)?;
        Ok(s)
    }

    pub fn push_entry(
        &mut self,
        dao: Dao,
        positions: &HashMap<Pubkey, Position>,
        pubkey: Pubkey,
    ) -> Result {
        match positions.get(&pubkey) {
            None => {
                return Err(Error::MissingPosition { position: pubkey });
            }
            Some(p) => match dao {
                Dao::Iot => {
                    self.balances.locked_iot += p.locked_tokens;
                    self.balances.veiot += p.voting_weight;
                    self.positions.veiot.push(pubkey);
                }
                Dao::Mobile => {
                    self.balances.locked_mobile += p.locked_tokens;
                    self.balances.vemobile += p.voting_weight;
                    self.positions.vemobile.push(pubkey);
                }
                Dao::Hnt => {
                    self.balances.locked_hnt += p.locked_tokens;
                    self.balances.vehnt.total += p.voting_weight;
                    self.positions.vehnt.push(pubkey);

                    if let Some(delegated) = &p.delegated {
                        match delegated.sub_dao {
                            SubDao::Iot => {
                                self.balances.vehnt.iot_delegated += p.voting_weight;
                                self.balances.pending_iot = delegated.pending_rewards;
                            }
                            SubDao::Mobile => {
                                self.balances.vehnt.mobile_delegated += p.voting_weight;
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
                        self.balances.vehnt.undelegated += p.voting_weight;
                    }
                }
            },
        }
        Ok(())
    }
}
