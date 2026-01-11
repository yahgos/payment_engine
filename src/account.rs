use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct ClientAccount {
    pub client: u16,
    #[serde(serialize_with = "round_to_four_decimals")]
    pub available: f64,
    #[serde(serialize_with = "round_to_four_decimals")]
    pub held: f64,
    #[serde(serialize_with = "round_to_four_decimals")]
    pub total: f64,
    pub locked: bool,
}

/// Rounds f64 to 4 decimal places for serialization
fn round_to_four_decimals<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let rounded = (value * 10000.0).round() / 10000.0;
    serializer.serialize_f64(rounded)
}

impl ClientAccount {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_account() {
        let account = ClientAccount::new(1);
        assert_eq!(account.client, 1);
        assert_eq!(account.available, 0.0);
        assert_eq!(account.total, 0.0);
        assert!(!account.locked);
    }

    #[test]
    fn test_precision() {
        let account = ClientAccount {
            client: 1,
            available: 1.23456789,
            held: 0.0,
            total: 1.23456789,
            locked: false,
        };

        let serialized = serde_json::to_string(&account).unwrap();
        assert!(serialized.contains("1.2346")); // Rounded to 4 decimals
    }
}
