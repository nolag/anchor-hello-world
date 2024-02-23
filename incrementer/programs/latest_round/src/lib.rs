use anchor_lang::prelude::*;
use std::mem::size_of;
use arrayref::{array_ref, array_refs};
use std::io::Write;

declare_id!("5iDtDHjXP7PcNbTQWSj8bfKm7tU6MhvsHGYXdqpcDZgR");

#[program]
pub mod latest_round {
    use super::*;

    pub fn create(_ctx: Context<Create>) -> Result<()> {
        let base_account = &mut _ctx.accounts.base_account;
        base_account.r1.median = 0;
        base_account.r2.median = 0;
        Ok(())
    }

    pub fn wire_emit_and_set_report(_ctx: Context<Set>, report: WireReport) -> Result<()> {
        wire_set_report(_ctx, report)?;
        emit!(ReportEvent{median: report.median});
        return Ok(())
    }

    pub fn wire_set_report(_ctx: Context<Set>, report: WireReport) -> Result<()> {
        let base = &mut _ctx.accounts.base_account;
        let slot = base.on;

        let prior = match slot {
            ReportSlot::ONE => {
                base.r1 = report;
                base.r1.median
            },
            _ => {
                base.r2 = report;
                base.r2.median
            },
        };

        if prior > report.median {
            base.on = ReportSlot::ONE;
        } else {
            base.on = ReportSlot::TWO;
        }

        Ok(())
    }

    pub fn set_report_raw<'info>(_ctx: Context<Set>, report: Report) -> Result<()> {
        set_report_from_raw(_ctx, report)
    }

    #[inline(never)]
    pub fn set_report<'info>(program_id: &Pubkey,accounts: &[AccountInfo<'info>], data: &[u8]) -> Result<()> {
        let mut bumps = std::collections::BTreeMap::new();
        let mut reallocs = std::collections::BTreeSet::new();
        // Deserialize accounts.
        let mut remaining_accounts: &[AccountInfo] = accounts;
        let mut accounts = Set::try_accounts(
            program_id,
            &mut remaining_accounts,
            data,
            &mut bumps,
            &mut reallocs,
        )?;

        // Construct a context
        let ctx = Context::new(program_id, &mut accounts, remaining_accounts, bumps);
        set_report_impl(ctx, data)
    }
}

#[inline(always)]
fn set_report_impl<'info>(ctx: Context<Set<'info>>, data: &[u8]) -> Result<()> {
    let rawreport = Report::unpack(data)?;
    set_report_from_raw(ctx, rawreport)
}

fn set_report_from_raw(ctx: Context<Set>, rawreport: Report) -> Result<()> {
    let base = &mut ctx.accounts.base_account;
    let slot = base.on;
    let report = WireReport {
        median: rawreport.median,
        observer_count: rawreport.observer_count,
        observers: rawreport.observers,
        observations_timestamp: rawreport.observations_timestamp,
        juels_per_lamport: rawreport.juels_per_lamport,
    };

    let prior = match slot {
        ReportSlot::ONE => {
            base.r1 = report;
            base.r1.median
        },
        _ => {
            base.r2 = report;
            base.r2.median
        },
    };

    if prior > report.median {
        base.on = ReportSlot::ONE;
    } else {
        base.on = ReportSlot::TWO;
    }

    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub enum ReportSlot {
    ONE,
    TWO
}

// Transaction Instructions
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = user, space = 16 + 112)]
    pub base_account: Account<'info, BaseAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Set<'info> {
    #[account(mut)]
    pub base_account: Account<'info, BaseAccount>,
}

// An account that goes inside a transaction instruction
#[account]
pub struct BaseAccount {
    pub r1: WireReport,
    pub r2: WireReport,
    pub on: ReportSlot,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Copy)]
pub struct WireReport {
    pub median: i128,
    pub observer_count: u8,
    pub observers: [u8; 19], // observer index
    pub observations_timestamp: u32,
    pub juels_per_lamport: u64,
}

#[event]
pub struct ReportEvent {
    pub median: i128,
}

#[derive(Clone, Default, Copy)]
pub struct Report {
    pub median: i128,
    pub observer_count: u8,
    pub observers: [u8; 19], // observer index
    pub observations_timestamp: u32,
    pub juels_per_lamport: u64,
}

impl Report {
    // (uint32, u8, bytes32, int128, u64)
    pub const LEN: usize =
        size_of::<u32>() + size_of::<u8>() + 32 + size_of::<i128>() + size_of::<u64>();

    pub fn unpack(raw_report: &[u8]) -> Result<Self> {
        let data = array_ref![raw_report, 0, Report::LEN];
        let (observations_timestamp, observer_count, observers, median, juels_per_lamport) =
            array_refs![data, 4, 1, 32, 16, 8];

        let observations_timestamp = u32::from_be_bytes(*observations_timestamp);
        let observer_count = observer_count[0];
        let observers = observers[..19].try_into().unwrap();
        let median = i128::from_be_bytes(*median);
        let juels_per_lamport = u64::from_be_bytes(*juels_per_lamport);

        Ok(Self {
            median,
            observer_count,
            observers,
            observations_timestamp,
            juels_per_lamport,
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut packed = Vec::with_capacity(Self::LEN);
        packed.extend_from_slice(&self.observations_timestamp.to_be_bytes());
        packed.push(self.observer_count);
        packed.extend_from_slice(&self.observers);
        packed.extend_from_slice(&self.median.to_be_bytes());
        packed.extend_from_slice(&self.juels_per_lamport.to_be_bytes());
        packed
    }
}

impl AnchorSerialize for Report {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::result::Result<(), std::io::Error> {
        let data = self.pack();
        writer.write_all(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl AnchorDeserialize for Report {
    fn deserialize(buf:&mut &[u8]) -> std::result::Result<Self, std::io::Error> {
        Self::unpack(buf).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to unpack Report"))
    }
}