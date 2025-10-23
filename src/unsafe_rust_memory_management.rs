use std::sync::Arc;
use std::thread;

struct SharedBuffer {
    data: Arc<[u8]>,
}

impl SharedBuffer {
    fn new(size: usize) -> Self {
        // Initialize bytes (here: zeros). No UB.
        let slice = vec![0u8; size].into_boxed_slice();
        Self { data: Arc::from(slice) }
    }

    fn get(&self, index: usize) -> Option<u8> {
        // Safe bounds-checked access, return by value (Copy)
        self.data.get(index).copied()
    }
}

fn main() {
    let buffer = Arc::new(SharedBuffer::new(1024));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let buf = buffer.clone();
            thread::spawn(move || {
                if let Some(val) = buf.get(i * 10) {
                    println!("Thread {} read: {}", i, val);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
