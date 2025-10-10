// Account data layout:
// [discriminator: u8][owner: Pubkey(32)][amount: u64][data_len: u32][data: Vec<u8>]

use std::mem;

#[derive(Debug, Clone)]
pub enum ParseError {
    InsufficientData,
    InvalidAlignment,
    InvalidDataLength,
}

#[repr(C)]
#[derive(Debug)]
pub struct AccountHeader {
    pub discriminator: u8,
    pub owner: [u8; 32],
    pub amount: u64,
    pub data_len: u32,
}

pub struct Account<'a> {
    pub header: &'a AccountHeader,
    pub data: &'a [u8],
}

impl<'a> Account<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, ParseError> {
        // Check if we have enough data for the header
        let header_size = mem::size_of::<AccountHeader>();
        if bytes.len() < header_size {
            return Err(ParseError::InsufficientData);
        }

        // Check alignment - the AccountHeader should be aligned to its most restrictive field
        // u64 requires 8-byte alignment
        let ptr = bytes.as_ptr() as usize;
        if ptr % 8 != 0 {
            return Err(ParseError::InvalidAlignment);
        }

        // Unsafe block to perform zero-copy deserialization
        // We've validated alignment and size, so this is safe
        let header = unsafe {
            &*(bytes.as_ptr() as *const AccountHeader)
        };

        // Validate the data length
        let data_len = header.data_len as usize;
        let expected_total_size = header_size + data_len;
        
        if bytes.len() < expected_total_size {
            return Err(ParseError::InvalidDataLength);
        }

        // Extract the data portion
        // The data starts right after the header, but we need to account for struct padding
        let data_start = mem::size_of::<AccountHeader>();
        let data = &bytes[data_start..data_start + data_len];
        

        Ok(Account { header, data })
    }

    pub fn discriminator(&self) -> u8 {
        self.header.discriminator
    }

    pub fn owner(&self) -> &[u8; 32] {
        &self.header.owner
    }

    pub fn amount(&self) -> u64 {
        self.header.amount
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }
}

pub fn run_zero_copy_deserialization() {
    println!("=== Zero-Copy Deserialization Example ===");
    
    // Create sample account data with proper alignment
    let header_size = mem::size_of::<AccountHeader>();
    let mut aligned_data = vec![0u8; header_size + 8]; // Extra space for alignment
    
    // Find the first 8-byte aligned position
    let ptr = aligned_data.as_ptr() as usize;
    let aligned_ptr = (ptr + 7) & !7; // Round up to next 8-byte boundary
    let offset = aligned_ptr - ptr;
    
    // Create the header at the aligned position
    let header = AccountHeader {
        discriminator: 1,
        owner: [0u8; 32],
        amount: 42,
        data_len: 5,
    };
    
    // Copy the header to the aligned position
    unsafe {
        std::ptr::copy_nonoverlapping(
            &header as *const AccountHeader as *const u8,
            aligned_data.as_mut_ptr().add(offset),
            header_size
        );
    }
    
    // Add some sample data right after the header
    let sample_data = b"Hello";
    let data_start = offset + header_size;
    aligned_data.resize(data_start + sample_data.len(), 0);
    aligned_data[data_start..data_start + sample_data.len()].copy_from_slice(sample_data);
    
    // Create a slice that starts at the aligned position and includes all the data
    let account_data = &aligned_data[offset..];
    
    println!("Created account data with {} bytes (aligned at offset {})", account_data.len(), offset);
    
    // Parse the account using zero-copy deserialization
    match Account::from_bytes(&account_data) {
        Ok(account) => {
            println!("Successfully parsed account:");
            println!("  Discriminator: {}", account.discriminator());
            println!("  Owner: {:?}", account.owner());
            println!("  Amount: {}", account.amount());
            println!("  Data: {:?}", String::from_utf8_lossy(account.data()));
        }
        Err(e) => {
            println!("Failed to parse account: {:?}", e);
        }
    }
    
    // Test error cases
    println!("\n=== Testing Error Cases ===");
    
    // Test insufficient data
    let short_data = &account_data[..10];
    match Account::from_bytes(short_data) {
        Ok(_) => println!("Unexpected success with short data"),
        Err(ParseError::InsufficientData) => println!("✓ Correctly detected insufficient data"),
        Err(e) => println!("Unexpected error: {:?}", e),
    }
    
    // Test invalid data length - create a new buffer with invalid data_len
    let mut invalid_aligned_data = vec![0u8; header_size + 8];
    let invalid_ptr = invalid_aligned_data.as_ptr() as usize;
    let invalid_aligned_ptr = (invalid_ptr + 7) & !7;
    let invalid_offset = invalid_aligned_ptr - invalid_ptr;
    
    let invalid_header = AccountHeader {
        discriminator: 1,
        owner: [0u8; 32],
        amount: 42,
        data_len: 1000, // Invalid data length
    };
    
    unsafe {
        std::ptr::copy_nonoverlapping(
            &invalid_header as *const AccountHeader as *const u8,
            invalid_aligned_data.as_mut_ptr().add(invalid_offset),
            header_size
        );
    }
    
    let invalid_account_data = &invalid_aligned_data[invalid_offset..];
    match Account::from_bytes(invalid_account_data) {
        Ok(_) => println!("Unexpected success with invalid data length"),
        Err(ParseError::InvalidDataLength) => println!("✓ Correctly detected invalid data length"),
        Err(e) => println!("Unexpected error: {:?}", e),
    }
}