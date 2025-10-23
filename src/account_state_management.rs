use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;

/// Represents the state of a Solana account
#[derive(Debug, Clone, PartialEq)]
pub struct AccountState {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: [u8; 32],
    pub executable: bool,
    pub rent_epoch: u64,
}

impl AccountState {
    pub fn new(lamports: u64, data: Vec<u8>, owner: [u8; 32]) -> Self {
        Self {
            lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        }
    }
}

/// Transaction ID type
pub type TransactionId = u64;

/// Represents a database transaction with rollback capabilities
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: TransactionId,
    pub block: u32,
    pub slot: u32,
    pub status: TransactionStatus,
    pub created_at: u64,
    pub locked_accounts: HashSet<[u8; 32]>,
    pub modifications: HashMap<[u8; 32], AccountState>, // Original state for rollback
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}

/// Error types for account state management
#[derive(Debug, Clone, PartialEq)]
pub enum AccountError {
    AccountNotFound,
    AccountLocked,
    TransactionNotFound,
    InvalidTransaction,
    InsufficientFunds,
    InvalidAccountData,
    ConcurrentModification,
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AccountError::AccountNotFound => write!(f, "Account not found"),
            AccountError::AccountLocked => write!(f, "Account is locked by another transaction"),
            AccountError::TransactionNotFound => write!(f, "Transaction not found"),
            AccountError::InvalidTransaction => write!(f, "Invalid transaction state"),
            AccountError::InsufficientFunds => write!(f, "Insufficient funds"),
            AccountError::InvalidAccountData => write!(f, "Invalid account data"),
            AccountError::ConcurrentModification => write!(f, "Concurrent modification detected"),
        }
    }
}

/// Write guard for account modifications within a transaction
pub struct AccountWriteGuard {
    pubkey: [u8; 32],
    account: AccountState,
    transaction_id: TransactionId,
    accounts_db: Arc<AccountsDb>,
}

impl AccountWriteGuard {
    pub fn get_lamports(&self) -> u64 {
        self.account.lamports
    }

    pub fn set_lamports(&mut self, lamports: u64) {
        self.account.lamports = lamports;
    }

    pub fn get_data(&self) -> &[u8] {
        &self.account.data
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.account.data = data;
    }

    pub fn get_owner(&self) -> [u8; 32] {
        self.account.owner
    }

    pub fn set_owner(&mut self, owner: [u8; 32]) {
        self.account.owner = owner;
    }

    pub fn transfer_lamports(&mut self, amount: u64) -> Result<(), AccountError> {
        if self.account.lamports < amount {
            return Err(AccountError::InsufficientFunds);
        }
        self.account.lamports -= amount;
        Ok(())
    }

    pub fn add_lamports(&mut self, amount: u64) {
        self.account.lamports += amount;
    }
}

impl Drop for AccountWriteGuard {
    fn drop(&mut self) {
        // Update the account in the transaction's modifications
        let mut transactions = self.accounts_db.transactions.write().unwrap();
        if let Some(transaction) = transactions.get_mut(&self.transaction_id) {
            transaction.modifications.insert(self.pubkey, self.account.clone());
        }
        
        // Release the lock for this account
        let mut locks = self.accounts_db.account_locks.write().unwrap();
        locks.remove(&self.pubkey);
    }
}

/// Main accounts database with transaction support
pub struct AccountsDb {
    accounts: Arc<RwLock<HashMap<[u8; 32], AccountState>>>,
    pub transactions: Arc<RwLock<HashMap<TransactionId, Transaction>>>,
    pub account_locks: Arc<RwLock<HashMap<[u8; 32], TransactionId>>>, // Maps account to locking transaction
    next_transaction_id: Arc<RwLock<TransactionId>>,
}

impl AccountsDb {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            account_locks: Arc::new(RwLock::new(HashMap::new())),
            next_transaction_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Begin a new transaction
    pub fn begin_transaction(&self, block: u32, slot: u32) -> Transaction {
        let id = {
            let mut next_id = self.next_transaction_id.write().unwrap();
            let current_id = *next_id;
            *next_id += 1;
            current_id
        };

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let transaction = Transaction {
            id,
            block,
            slot,
            status: TransactionStatus::Active,
            created_at,
            locked_accounts: HashSet::new(),
            modifications: HashMap::new(),
        };

        {
            let mut transactions = self.transactions.write().unwrap();
            transactions.insert(id, transaction.clone());
        }

        transaction
    }

    /// Load an account for write access with pessimistic locking
    pub fn load_account_for_write(&self, pubkey: &[u8; 32], tx: &Transaction) 
        -> Result<AccountWriteGuard, AccountError> {
        
        // Check if account is already locked by another transaction
        {
            let locks = self.account_locks.read().unwrap();
            if let Some(&locking_tx_id) = locks.get(pubkey) {
                if locking_tx_id != tx.id {
                    return Err(AccountError::AccountLocked);
                }
            }
        }

        // Lock the account for this transaction
        {
            let mut locks = self.account_locks.write().unwrap();
            locks.insert(*pubkey, tx.id);
        }

        // Get the current account state
        let account = {
            let accounts = self.accounts.read().unwrap();
            accounts.get(pubkey).cloned()
                .unwrap_or_else(|| AccountState::new(0, Vec::new(), [0; 32]))
        };

        // Store original state for rollback if not already stored
        {
            let mut transactions = self.transactions.write().unwrap();
            if let Some(transaction) = transactions.get_mut(&tx.id) {
                if !transaction.modifications.contains_key(pubkey) {
                    transaction.modifications.insert(*pubkey, account.clone());
                }
                transaction.locked_accounts.insert(*pubkey);
            }
        }

        Ok(AccountWriteGuard {
            pubkey: *pubkey,
            account,
            transaction_id: tx.id,
            accounts_db: Arc::new(AccountsDb {
                accounts: Arc::clone(&self.accounts),
                transactions: Arc::clone(&self.transactions),
                account_locks: Arc::clone(&self.account_locks),
                next_transaction_id: Arc::clone(&self.next_transaction_id),
            }),
        })
    }

    /// Commit a transaction atomically
    pub fn commit_transaction(&self, tx: Transaction) -> Result<(), AccountError> {
        let mut transactions = self.transactions.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        let mut locks = self.account_locks.write().unwrap();

        // Verify transaction is still active
        if let Some(stored_tx) = transactions.get(&tx.id) {
            if stored_tx.status != TransactionStatus::Active {
                return Err(AccountError::InvalidTransaction);
            }
        } else {
            return Err(AccountError::TransactionNotFound);
        }

        // Apply all modifications atomically
        for (pubkey, account_state) in &tx.modifications {
            accounts.insert(*pubkey, account_state.clone());
        }

        // Release all locks held by this transaction
        for pubkey in &tx.locked_accounts {
            locks.remove(pubkey);
        }

        // Mark transaction as committed
        if let Some(transaction) = transactions.get_mut(&tx.id) {
            transaction.status = TransactionStatus::Committed;
        }

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback_transaction(&self, tx: Transaction) -> Result<(), AccountError> {
        let mut transactions = self.transactions.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        let mut locks = self.account_locks.write().unwrap();

        // Verify transaction exists and is active
        if let Some(stored_tx) = transactions.get(&tx.id) {
            if stored_tx.status != TransactionStatus::Active {
                return Err(AccountError::InvalidTransaction);
            }
        } else {
            return Err(AccountError::TransactionNotFound);
        }

        // Restore original states
        for (pubkey, original_state) in &tx.modifications {
            accounts.insert(*pubkey, original_state.clone());
        }

        // Release all locks held by this transaction
        for pubkey in &tx.locked_accounts {
            locks.remove(pubkey);
        }

        // Mark transaction as aborted
        if let Some(transaction) = transactions.get_mut(&tx.id) {
            transaction.status = TransactionStatus::Aborted;
        }

        Ok(())
    }

    /// Get a transaction by ID
    pub fn get_transaction(&self, tx_id: TransactionId) -> Result<Transaction, AccountError> {
        let transactions = self.transactions.read().unwrap();
        transactions.get(&tx_id)
            .cloned()
            .ok_or(AccountError::TransactionNotFound)
    }

    /// Get account state (read-only)
    pub fn get_account(&self, pubkey: &[u8; 32]) -> Option<AccountState> {
        let accounts = self.accounts.read().unwrap();
        accounts.get(pubkey).cloned()
    }

    /// Create a new account
    pub fn create_account(&self, pubkey: [u8; 32], account: AccountState) {
        let mut accounts = self.accounts.write().unwrap();
        accounts.insert(pubkey, account);
    }
}

pub fn run_account_state_management() {
    println!("=== Account State Management Example ===");
    
    let db = AccountsDb::new();
    
    // Create some test accounts
    let alice_pubkey = [1u8; 32];
    let bob_pubkey = [2u8; 32];
    let charlie_pubkey = [3u8; 32];
    
    let alice_account = AccountState::new(1000, b"Alice's data".to_vec(), [0u8; 32]);
    let bob_account = AccountState::new(500, b"Bob's data".to_vec(), [0u8; 32]);
    let charlie_account = AccountState::new(200, b"Charlie's data".to_vec(), [0u8; 32]);
    
    db.create_account(alice_pubkey, alice_account);
    db.create_account(bob_pubkey, bob_account);
    db.create_account(charlie_pubkey, charlie_account);
    
    println!("Initial account states:");
    println!("Alice: {} lamports", db.get_account(&alice_pubkey).unwrap().lamports);
    println!("Bob: {} lamports", db.get_account(&bob_pubkey).unwrap().lamports);
    println!("Charlie: {} lamports", db.get_account(&charlie_pubkey).unwrap().lamports);
    
    // Start a transaction
    let tx = db.begin_transaction(1, 100);
    println!("\nStarted transaction {}", tx.id);
    
    // Load accounts for modification
    let mut alice_guard = db.load_account_for_write(&alice_pubkey, &tx).unwrap();
    let mut bob_guard = db.load_account_for_write(&bob_pubkey, &tx).unwrap();
    
    println!("Loaded Alice and Bob accounts for write");
    
    // Perform transfers
    alice_guard.transfer_lamports(100).unwrap();
    bob_guard.add_lamports(100);
    
    println!("Transferred 100 lamports from Alice to Bob");
    
    // Drop guards to apply changes
    drop(alice_guard);
    drop(bob_guard);
    
    // Commit the transaction
    match db.commit_transaction(tx) {
        Ok(()) => println!("Transaction committed successfully"),
        Err(e) => println!("Failed to commit transaction: {}", e),
    }
    
    println!("\nAccount states after transaction:");
    println!("Alice: {} lamports", db.get_account(&alice_pubkey).unwrap().lamports);
    println!("Bob: {} lamports", db.get_account(&bob_pubkey).unwrap().lamports);
    println!("Charlie: {} lamports", db.get_account(&charlie_pubkey).unwrap().lamports);
    
    // Demonstrate rollback scenario
    println!("\n=== Rollback Scenario ===");
    let tx2 = db.begin_transaction(2, 101);
    println!("Started transaction {}", tx2.id);
    
    let mut charlie_guard = db.load_account_for_write(&charlie_pubkey, &tx2).unwrap();
    charlie_guard.transfer_lamports(50).unwrap();
    drop(charlie_guard);
    
    println!("Charlie transferred 50 lamports (will be rolled back)");
    println!("Charlie before rollback: {} lamports", db.get_account(&charlie_pubkey).unwrap().lamports);
    
    // Rollback the transaction
    match db.rollback_transaction(tx2) {
        Ok(()) => println!("Transaction rolled back successfully"),
        Err(e) => println!("Failed to rollback transaction: {}", e),
    }
    
    println!("Charlie after rollback: {} lamports", db.get_account(&charlie_pubkey).unwrap().lamports);
    
    // Demonstrate concurrent access protection
    println!("\n=== Concurrent Access Protection ===");
    let tx3 = db.begin_transaction(3, 102);
    let tx4 = db.begin_transaction(4, 103);
    
    // First transaction locks Alice
    let _alice_guard = db.load_account_for_write(&alice_pubkey, &tx3).unwrap();
    println!("Transaction {} locked Alice", tx3.id);
    
    // Second transaction tries to lock Alice (should fail)
    match db.load_account_for_write(&alice_pubkey, &tx4) {
        Ok(_) => println!("ERROR: Should not be able to lock Alice twice!"),
        Err(AccountError::AccountLocked) => println!("Transaction {} correctly blocked from locking Alice", tx4.id),
        Err(e) => println!("Unexpected error: {}", e),
    }
    
    drop(_alice_guard);
    
    // Now try to lock Alice with the second transaction (should succeed)
    match db.load_account_for_write(&alice_pubkey, &tx4) {
        Ok(_) => println!("Transaction {} successfully locked Alice after first transaction released it", tx4.id),
        Err(e) => println!("Unexpected error after release: {}", e),
    }
    
    db.rollback_transaction(tx3).unwrap();
    db.rollback_transaction(tx4).unwrap();
    
    println!("\nAccount state management demonstration completed!");
}