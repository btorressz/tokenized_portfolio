use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use solana_program::clock::Clock;

declare_id!("6SdCjCTYtGeAzcKquDFAm5C5pMEayJhtmxWvmPmp1BXP");

#[program]
pub mod tokenized_portfolio {
    use super::*;

    // Initialize a new portfolio with an owner
    pub fn initialize_portfolio(ctx: Context<InitializePortfolio>) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;
        portfolio.owner = ctx.accounts.owner.key();
        portfolio.total_value = 0;
        portfolio.historical_values = vec![];
        portfolio.last_update_timestamp = Clock::get()?.unix_timestamp;
        portfolio.min_value_threshold = 0;
        portfolio.max_value_threshold = u64::MAX;
        portfolio.management_fee = 0;
        portfolio.performance_fee = 0;
        portfolio.total_shares = 1_000_000; // Initial shares for tokenized portfolio
        Ok(())
    }

    // Add an asset to the portfolio
    pub fn add_asset(
        ctx: Context<AddAsset>,
        asset_symbol: String,
        asset_amount: u64,
        asset_value: u64,
    ) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;
        portfolio.total_value += asset_value;

        let asset = Asset {
            symbol: asset_symbol.clone(),
            amount: asset_amount,
            value: asset_value,
        };
        portfolio.assets.push(asset);

        emit!(AssetUpdated {
            owner: portfolio.owner,
            asset_symbol,
            old_value: 0,
            new_value: asset_value,
        });

        Ok(())
    }

    // Transfer assets between token accounts using CPI with Solana's Token Program
    pub fn transfer_asset(ctx: Context<TransferAsset>, asset_symbol: String, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let asset = portfolio
            .assets
            .iter_mut()
            .find(|a| a.symbol == asset_symbol)
            .ok_or(PortfolioError::AssetNotFound)?;

        if asset.amount < amount {
            return Err(PortfolioError::InsufficientBalance.into());
        }

        asset.amount -= amount;

        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.destination_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    // Update the value of an asset
    pub fn update_asset_value(ctx: Context<UpdateAssetValue>, asset_symbol: String, new_value: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let asset_index = portfolio
            .assets
            .iter()
            .position(|a| a.symbol == asset_symbol)
            .ok_or(PortfolioError::AssetNotFound)?;

        let old_value = portfolio.assets[asset_index].value;

        let new_total_value = portfolio.total_value - old_value + new_value;

        let asset = &mut portfolio.assets[asset_index];
        asset.value = new_value;
        portfolio.total_value = new_total_value;

        emit!(AssetUpdated {
            owner: portfolio.owner,
            asset_symbol,
            old_value,
            new_value,
        });

        Ok(())
    }

    // Record portfolio performance by taking a snapshot of the total value
    pub fn record_performance(ctx: Context<RecordPerformance>, current_value: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;
        portfolio.historical_values.push(current_value);
        portfolio.last_update_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }

    // Corrected: Rebalance the portfolio based on target ratios for each asset
    pub fn rebalance_portfolio(ctx: Context<RebalancePortfolio>, target_ratios: Vec<(String, u64)>) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let total_value = portfolio.total_value;

        for (symbol, target_ratio) in target_ratios.iter() {
            if let Some(asset) = portfolio.assets.iter_mut().find(|a| &a.symbol == symbol) {
                let target_value = total_value * target_ratio / 100;
                asset.value = target_value;
            }
        }

        Ok(())
    }

    // Withdraw an asset from the portfolio
    pub fn withdraw_asset(ctx: Context<WithdrawAsset>, asset_symbol: String, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let asset = portfolio
            .assets
            .iter_mut()
            .find(|a| a.symbol == asset_symbol)
            .ok_or(PortfolioError::AssetNotFound)?;

        if asset.amount < amount {
            return Err(PortfolioError::InsufficientBalance.into());
        }

        asset.amount -= amount;
        portfolio.total_value -= amount * asset.value;

        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.destination_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    // Check if the portfolio violates risk thresholds and rebalance
    pub fn check_risk(ctx: Context<CheckRisk>) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        if portfolio.total_value < portfolio.min_value_threshold {
            msg!("Portfolio under minimum value. Consider rebalancing.");
            return Err(PortfolioError::UnderMinValue.into());
        }

        if portfolio.total_value > portfolio.max_value_threshold {
            msg!("Portfolio exceeds maximum threshold. Rebalancing.");
        }

        Ok(())
    }

    // Update asset value using a price from a decentralized oracle
    pub fn update_asset_value_with_oracle(ctx: Context<UpdateAssetWithOracle>, asset_symbol: String) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let asset_index = portfolio
            .assets
            .iter()
            .position(|a| a.symbol == asset_symbol)
            .ok_or(PortfolioError::AssetNotFound)?;

        let old_value = portfolio.assets[asset_index].value;

        let oracle_price = get_oracle_price(&ctx.accounts.oracle_account)?;

        let new_total_value = portfolio.total_value - old_value + oracle_price;

        let asset = &mut portfolio.assets[asset_index];
        asset.value = oracle_price;
        portfolio.total_value = new_total_value;

        emit!(AssetUpdated {
            owner: portfolio.owner,
            asset_symbol,
            old_value,
            new_value: oracle_price,
        });

        Ok(())
    }

    // Apply custom management and performance fees
    pub fn apply_fees(ctx: Context<ApplyFees>) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let management_fee = portfolio.total_value * portfolio.management_fee / 100;
        let performance_fee = calculate_performance_fee(&portfolio)?;

        portfolio.total_value -= management_fee + performance_fee;

        emit!(FeesApplied {
            owner: portfolio.owner,
            management_fee,
            performance_fee,
        });

        Ok(())
    }

    // Feature: Stake tokens for rewards
    pub fn stake_tokens(ctx: Context<StakeTokens>, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;
        let user_stake = &mut ctx.accounts.user_stake;

        user_stake.amount += amount;
        user_stake.last_reward_claim_timestamp = Clock::get()?.unix_timestamp;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.stake_pool_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    // Feature: Provide liquidity to a decentralized pool
    pub fn provide_liquidity(ctx: Context<ProvideLiquidity>, asset_symbol: String, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let asset = portfolio
            .assets
            .iter_mut()
            .find(|a| a.symbol == asset_symbol)
            .ok_or(PortfolioError::AssetNotFound)?;

        if asset.amount < amount {
            return Err(PortfolioError::InsufficientBalance.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.liquidity_pool_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    // Dynamic fees based on performance
    pub fn apply_dynamic_fees(ctx: Context<ApplyFees>, performance_bonus_threshold: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        let base_management_fee = portfolio.total_value * portfolio.management_fee / 100;
        let base_performance_fee = calculate_performance_fee(&portfolio)?;

        let performance_bonus = if portfolio.total_value > performance_bonus_threshold {
            portfolio.total_value * 5 / 100
        } else {
            0
        };

        let total_performance_fee = base_performance_fee + performance_bonus;
        portfolio.total_value -= base_management_fee + total_performance_fee;

        emit!(FeesApplied {
            owner: portfolio.owner,
            management_fee: base_management_fee,
            performance_fee: total_performance_fee,
        });

        Ok(())
    }

    // Automatic rebalancing
    pub fn rebalance_automatically(ctx: Context<RebalancePortfolio>, target_ratios: Vec<(String, u64)>) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        if portfolio.total_value > portfolio.max_value_threshold || portfolio.total_value < portfolio.min_value_threshold {
            return rebalance_portfolio(ctx, target_ratios);
        }

        Ok(())
    }

    // Feature: Distribute rewards based on staking
    pub fn distribute_staking_rewards(ctx: Context<DistributeRewards>, reward_amount: u64) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;

        let staking_duration = current_time - user_stake.last_reward_claim_timestamp;
        let reward = reward_amount * staking_duration as u64 / 1_000_000;

        user_stake.last_reward_claim_timestamp = current_time;

        Ok(())
    }

    // Flash loan implementation
    pub fn take_flash_loan(ctx: Context<FlashLoan>, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        msg!("Flash loan of {} provided.", amount);

        // Logic for repayment within the same transaction would go here.

        Ok(())
    }

    // Governance token issuance
    pub fn issue_governance_tokens(ctx: Context<IssueGovernanceTokens>, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        msg!("{} governance tokens issued.", amount);

        // Logic for issuing governance tokens to portfolio holders would go here.

        Ok(())
    }

    // Multi-signature withdrawal approval
    pub fn withdraw_with_multisig(ctx: Context<WithdrawWithMultisig>, amount: u64) -> Result<()> {
        let portfolio = &mut ctx.accounts.portfolio;

        msg!("Multi-signature approval for withdrawal of {}.", amount);

        // Multi-signature logic to approve the transaction.

        Ok(())
    }
}

// Account context definitions
#[derive(Accounts)]
pub struct InitializePortfolio<'info> {
    #[account(init, payer = owner, space = 8 + Portfolio::MAX_SIZE)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddAsset<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferAsset<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_account: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawAsset<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_account: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RecordPerformance<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RebalancePortfolio<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CheckRisk<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAssetValue<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAssetWithOracle<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
    pub oracle_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ApplyFees<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

// Added Account context for staking rewards distribution
#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub user_stake: Account<'info, UserStake>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub stake_pool_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_stake: Account<'info, UserStake>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ProvideLiquidity<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub liquidity_pool_account: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FlashLoan<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct IssueGovernanceTokens<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawWithMultisig<'info> {
    #[account(mut, has_one = owner)]
    pub portfolio: Account<'info, Portfolio>,
    pub owner: Signer<'info>,
}

// Portfolio structure for managing tokenized assets
#[account]
pub struct Portfolio {
    pub owner: Pubkey,
    pub total_value: u64,
    pub total_shares: u64,
    pub assets: Vec<Asset>,
    pub historical_values: Vec<u64>,
    pub last_update_timestamp: i64,
    pub min_value_threshold: u64,
    pub max_value_threshold: u64,
    pub management_fee: u64,
    pub performance_fee: u64,
}

impl Portfolio {
    const MAX_SIZE: usize = 32 + 8 + (4 + 64 * (32 + 8 + 8)) + 4 + (8 * 100) + 8 + 8 + 8 + 8;
}

#[account]
pub struct UserStake {
    pub owner: Pubkey,
    pub amount: u64,
    pub last_reward_claim_timestamp: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Asset {
    pub symbol: String,
    pub amount: u64,
    pub value: u64,
}

// Custom error codes for portfolio management
#[error_code]
pub enum PortfolioError {
    #[msg("The asset was not found in the portfolio.")]
    AssetNotFound,
    #[msg("Insufficient balance for the transfer.")]
    InsufficientBalance,
    #[msg("Portfolio value is below the minimum threshold.")]
    UnderMinValue,
}

// Event logging for asset updates and fees
#[event]
pub struct AssetUpdated {
    pub owner: Pubkey,
    pub asset_symbol: String,
    pub old_value: u64,
    pub new_value: u64,
}

#[event]
pub struct FeesApplied {
    pub owner: Pubkey,
    pub management_fee: u64,
    pub performance_fee: u64,
}

// Placeholder for oracle price fetching logic
fn get_oracle_price(oracle_account: &AccountInfo) -> Result<u64> {
    Ok(100)
}

// Placeholder function to calculate performance fee
fn calculate_performance_fee(portfolio: &Portfolio) -> Result<u64> {
    Ok(0)
}
