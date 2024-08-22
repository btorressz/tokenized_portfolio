# tokenized_portfolio

This project implements a **Tokenized Portfolio** smart contract using the Solana blockchain, built with the **Anchor** framework. The program allows users to create and manage portfolios of tokenized assets, perform asset transfers, rebalance portfolios, apply custom management and performance fees, stake tokens, and more.
This project was made in Solana Playground IDE and Replit.

## Features

### Core Portfolio Management

- **Initialize Portfolio**: Create a new tokenized portfolio with an owner and default configuration.
- **Add Asset**: Add a new asset to the portfolio with a specified amount and value.
- **Transfer Assets**: Transfer assets between token accounts using Solana's Token Program (via CPI).
- **Update Asset Value**: Manually update the value of an asset or use a decentralized oracle.
- **Withdraw Assets**: Withdraw assets from the portfolio to external accounts.
- **Rebalance Portfolio**: Automatically rebalance the portfolio based on predefined asset ratios.
- **Check Risk**: Check if the portfolio violates predefined risk thresholds (min/max value).

  ### Fees

- **Apply Management and Performance Fees**: Automatically apply fees based on portfolio performance.
- - **Dynamic Fees**: Apply dynamic fees that change based on the portfolio's performance thresholds.
 
### Advanced Features

- **Stake Tokens**: Stake tokens into the portfolio’s staking pool and earn rewards.
- **Distribute Staking Rewards**: Distribute rewards to users based on their staking duration.
- **Provide Liquidity**: Provide liquidity to decentralized pools with assets from the portfolio.
- **Flash Loans**: Take out flash loans from the portfolio with repayment required within the same transaction.
- **Governance Tokens**: Issue governance tokens to portfolio holders.
- **Multi-Signature Withdrawal**: Enable multi-signature approval for asset withdrawals from the portfolio.

  ## Project Structure

- **lib.rs**: Contains all the smart contract logic and instructions for portfolio management.
- **tests.rs**:Contains detailed integration tests designed to rigorously verify the contract.



