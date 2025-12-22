use crate::trade::info::TransactionInfo;

impl TransactionInfo {
    /// Get total SOL received by signer (in lamports)
    /// Includes all SOL transferred to the signer's account
    pub fn get_signer_total_sol_received_lamports(&self) -> u64 {
        let signer_index = self
            .raw_account_keys
            .iter()
            .position(|addr| addr == &self.signer)
            .or_else(|| {
                self.raw_account_keys
                    .iter()
                    .position(|addr| addr == &self.fee_payer)
            })
            .unwrap_or(0);

        if signer_index >= self.raw_pre_balances.len()
            || signer_index >= self.raw_post_balances.len()
        {
            return 0;
        }
        let mut total_inflows = 0u64;
        for i in 0..self.raw_account_keys.len() {
            if i == signer_index {
                continue;
            }
            if i < self.raw_pre_balances.len() && i < self.raw_post_balances.len() {
                let pre = self.raw_pre_balances[i];
                let post = self.raw_post_balances[i];
                // If other account balance decreased
                if post < pre {
                    total_inflows += pre - post;
                }
            }
        }
        let signer_pre = self.raw_pre_balances[signer_index];
        let signer_post = self.raw_post_balances[signer_index];
        if signer_post > signer_pre {
            let net_increase = signer_post - signer_pre;
            return net_increase + self.get_signer_total_sol_paid_lamports();
        } else {
            return 0;
        }
    }

    /// Get total SOL received by signer (in SOL, decimal)
    pub fn get_signer_total_sol_received_sol(&self) -> f64 {
        use solana_sdk::native_token::LAMPORTS_PER_SOL;
        let lamports = self.get_signer_total_sol_received_lamports();
        lamports as f64 / LAMPORTS_PER_SOL as f64
    }

    /// Get total SOL paid by signer (in lamports)
    /// Includes all SOL transferred from signer's account, including transaction fees
    pub fn get_signer_total_sol_paid_lamports(&self) -> u64 {
        let signer_index = self
            .raw_account_keys
            .iter()
            .position(|addr| addr == &self.signer)
            .or_else(|| {
                self.raw_account_keys
                    .iter()
                    .position(|addr| addr == &self.fee_payer)
            })
            .unwrap_or(0);
        if signer_index >= self.raw_pre_balances.len()
            || signer_index >= self.raw_post_balances.len()
        {
            return 0;
        }
        let mut total_outflows = 0u64;
        for i in 0..self.raw_account_keys.len() {
            if i == signer_index {
                continue;
            }
            if i < self.raw_pre_balances.len() && i < self.raw_post_balances.len() {
                let pre = self.raw_pre_balances[i];
                let post = self.raw_post_balances[i];
                // If other account balance increased
                if post > pre {
                    total_outflows += post - pre;
                }
            }
        }
        total_outflows
    }

    /// Get total SOL paid by signer (in SOL, decimal)
    pub fn get_signer_total_sol_paid_sol(&self) -> f64 {
        use solana_sdk::native_token::LAMPORTS_PER_SOL;
        let lamports = self.get_signer_total_sol_paid_lamports();
        lamports as f64 / LAMPORTS_PER_SOL as f64
    }

    /// Get signer's net SOL income (in lamports)
    /// Total income - total expenses, positive means net income, negative means net expense
    pub fn get_signer_net_sol_income_lamports(&self) -> i64 {
        let total_received = self.get_signer_total_sol_received_lamports() as i64;
        let total_paid = self.get_signer_total_sol_paid_lamports() as i64;
        total_received - total_paid
    }

    /// Get signer's net SOL income (in SOL, decimal)
    /// Total income - total expenses, positive means net income, negative means net expense
    pub fn get_signer_net_sol_income_sol(&self) -> f64 {
        use solana_sdk::native_token::LAMPORTS_PER_SOL;
        let lamports = self.get_signer_net_sol_income_lamports();
        lamports as f64 / LAMPORTS_PER_SOL as f64
    }

    /// Get signer's net SOL expense (in lamports)
    /// Total expenses - total income, positive means net expense, negative means net income
    pub fn get_signer_net_sol_expense_lamports(&self) -> i64 {
        let total_paid = self.get_signer_total_sol_paid_lamports() as i64;
        let total_received = self.get_signer_total_sol_received_lamports() as i64;
        total_paid - total_received
    }

    /// Get signer's net SOL expense (in SOL, decimal)
    /// Total expenses - total income, positive means net expense, negative means net income
    pub fn get_signer_net_sol_expense_sol(&self) -> f64 {
        use solana_sdk::native_token::LAMPORTS_PER_SOL;
        let lamports = self.get_signer_net_sol_expense_lamports();
        lamports as f64 / LAMPORTS_PER_SOL as f64
    }
}
