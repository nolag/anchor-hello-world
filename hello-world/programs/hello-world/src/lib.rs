use anchor_lang::prelude::*;

declare_id!("7shjKeZsSsfkrYp7gXacjXzzVywDivkJfzTrAG4ZkZi4");

#[program]
pub mod hello_world {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
