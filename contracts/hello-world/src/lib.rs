#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, log, symbol_short, Address, Env, String, Symbol,
};

// Structure to store brand information
#[contracttype]
#[derive(Clone)]
pub struct Brand {
    pub brand_id: u64,
    pub brand_name: String,
    pub is_active: bool,
}

// Mapping for brands
#[contracttype]
pub enum BrandBook {
    Brand(u64),
}

// Counter for brands
const BRAND_COUNT: Symbol = symbol_short!("B_COUNT");

// Mapping for user balances: (User, Brand) -> Balance
#[contracttype]
pub enum UserBalance {
    Balance(Address, u64),
}

#[contract]
pub struct LoyaltyTokenExchange;

#[contractimpl]
impl LoyaltyTokenExchange {
    /// Register a new brand in the exchange platform
    /// Returns the brand_id of the newly registered brand
    pub fn register_brand(env: Env, brand_name: String) -> u64 {
        // Get current brand count or start from 0
        let mut brand_count: u64 = env.storage().instance().get(&BRAND_COUNT).unwrap_or(0);
        brand_count += 1;

        // Create new brand instance
        let new_brand = Brand {
            brand_id: brand_count,
            brand_name: brand_name.clone(),
            is_active: true,
        };

        // Store the brand
        env.storage()
            .instance()
            .set(&BrandBook::Brand(brand_count), &new_brand);
        env.storage().instance().set(&BRAND_COUNT, &brand_count);
        env.storage().instance().extend_ttl(100000, 100000);

        log!(&env, "✅ Brand registered with ID: {}", brand_count);
        brand_count
    }

    /// Issue loyalty tokens to a user from a specific brand
    pub fn issue_tokens(env: Env, user: Address, brand_id: u64, amount: i64) {
        user.require_auth();

        // Verify brand exists and is active
        let brand = Self::view_brand(env.clone(), brand_id);
        if !brand.is_active {
            panic!("Brand is not active");
        }
        if amount <= 0 {
            panic!("Amount must be positive");
        }

        // Update user balance
        let balance_key = UserBalance::Balance(user.clone(), brand_id);
        let current_balance: i64 = env.storage().instance().get(&balance_key).unwrap_or(0);
        let new_balance = current_balance + amount;
        env.storage().instance().set(&balance_key, &new_balance);
        env.storage().instance().extend_ttl(100000, 100000);

        log!(
            &env,
            "✅ Issued {} tokens from brand {} to user",
            amount,
            brand_id
        );
    }

    /// Exchange tokens between two brands (1:1 ratio)
    pub fn exchange_tokens(env: Env, user: Address, from_brand: u64, to_brand: u64, amount: i64) {
        user.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }
        if from_brand == to_brand {
            panic!("Cannot exchange to the same brand");
        }

        // Check both brands
        let from_brand_data = Self::view_brand(env.clone(), from_brand);
        let to_brand_data = Self::view_brand(env.clone(), to_brand);
        if !from_brand_data.is_active || !to_brand_data.is_active {
            panic!("One or both brands are not active");
        }

        // Deduct from source balance
        let from_balance_key = UserBalance::Balance(user.clone(), from_brand);
        let from_balance: i64 = env.storage().instance().get(&from_balance_key).unwrap_or(0);
        if from_balance < amount {
            panic!("Insufficient balance");
        }

        let new_from_balance = from_balance - amount;
        env.storage().instance().set(&from_balance_key, &new_from_balance);

        // Add to destination
        let to_balance_key = UserBalance::Balance(user.clone(), to_brand);
        let to_balance: i64 = env.storage().instance().get(&to_balance_key).unwrap_or(0);
        let new_to_balance = to_balance + amount;
        env.storage().instance().set(&to_balance_key, &new_to_balance);
        env.storage().instance().extend_ttl(100000, 100000);

        log!(
            &env,
            "✅ Exchanged {} tokens from brand {} → brand {}",
            amount,
            from_brand,
            to_brand
        );
    }

    /// View user's token balance
    pub fn view_user_balance(env: Env, user: Address, brand_id: u64) -> i64 {
        let balance_key = UserBalance::Balance(user, brand_id);
        env.storage().instance().get(&balance_key).unwrap_or(0)
    }

    /// View brand details by brand_id
    pub fn view_brand(env: Env, brand_id: u64) -> Brand {
        let key = BrandBook::Brand(brand_id);
        env.storage().instance().get(&key).unwrap_or(Brand {
            brand_id: 0,
            brand_name: String::from_str(&env, "Not_Found"),
            is_active: false,
        })
    }

    /// Get total number of registered brands
    pub fn get_brand_count(env: Env) -> u64 {
        env.storage().instance().get(&BRAND_COUNT).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    #[test]
    fn test_register_brand() {
        let env = Env::default();
        let contract_id = env.register(None, LoyaltyTokenExchange);
        let client = LoyaltyTokenExchangeClient::new(&env, &contract_id);

        let brand_name = String::from_str(&env, "Starbucks");
        let brand_id = client.register_brand(&brand_name);

        assert_eq!(brand_id, 1);
        let brand = client.view_brand(&brand_id);
        assert_eq!(brand.brand_name, brand_name);
        assert!(brand.is_active);
    }

    #[test]
    fn test_issue_and_view_tokens() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(None, LoyaltyTokenExchange);
        let client = LoyaltyTokenExchangeClient::new(&env, &contract_id);

        let user = Address::generate(&env);
        let brand_name = String::from_str(&env, "Nike");
        let brand_id = client.register_brand(&brand_name);

        client.issue_tokens(&user, &brand_id, &1000);
        let balance = client.view_user_balance(&user, &brand_id);
        assert_eq!(balance, 1000);
    }

    #[test]
    fn test_exchange_tokens() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(None, LoyaltyTokenExchange);
        let client = LoyaltyTokenExchangeClient::new(&env, &contract_id);

        let user = Address::generate(&env);

        let brand1 = String::from_str(&env, "Amazon");
        let brand2 = String::from_str(&env, "Apple");

        let brand_id_1 = client.register_brand(&brand1);
        let brand_id_2 = client.register_brand(&brand2);

        client.issue_tokens(&user, &brand_id_1, &1000);
        client.exchange_tokens(&user, &brand_id_1, &brand_id_2, &500);

        let balance1 = client.view_user_balance(&user, &brand_id_1);
        let balance2 = client.view_user_balance(&user, &brand_id_2);

        assert_eq!(balance1, 500);
        assert_eq!(balance2, 500);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn test_exchange_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(None, LoyaltyTokenExchange);
        let client = LoyaltyTokenExchangeClient::new(&env, &contract_id);

        let user = Address::generate(&env);

        let brand1 = String::from_str(&env, "Tesla");
        let brand2 = String::from_str(&env, "SpaceX");

        let brand_id_1 = client.register_brand(&brand1);
        let brand_id_2 = client.register_brand(&brand2);

        client.issue_tokens(&user, &brand_id_1, &100);
        client.exchange_tokens(&user, &brand_id_1, &brand_id_2, &500);
    }
}
