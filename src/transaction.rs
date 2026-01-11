use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

impl Transaction {
    /// Returns true if this transaction type requires an amount
    pub fn requires_amount(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Deposit | TransactionType::Withdrawal
        )
    }

    /// Returns true if this transaction type is a dispute-related action
    pub fn is_dispute_action(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback
        )
    }

    /// Validates that the transaction has required fields
    pub fn is_valid(&self) -> bool {
        if self.requires_amount() {
            self.amount.is_some() && self.amount.unwrap() > 0.0
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_amount() {
        let deposit = Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(100.0),
        };
        assert!(deposit.requires_amount());

        let dispute = Transaction {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };
        assert!(!dispute.requires_amount());
    }

    #[test]
    fn test_is_valid() {
        let valid = Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(100.0),
        };
        assert!(valid.is_valid());

        let invalid = Transaction {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(0.0),
        };
        assert!(!invalid.is_valid());
    }
}
