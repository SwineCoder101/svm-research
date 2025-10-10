use std::sync::Arc;
use std::thread;

struct SharedBuffer {
    data: *mut u8,
    len: usize,
}

impl SharedBuffer {
    fn new(size: usize) -> Self {
        let layout = std::alloc::Layout::array::<u8>(size).unwrap();
        let data = unsafe { std::alloc::alloc(layout) };
        Self { data, len: size }
    }

    fn get(&self, index: usize) -> Option<&u8> {
        if index < self.len {
            unsafe { Some(&*self.data.add(index)) }
        } else {
            None
        }
    }
}

fn main() {
    let buffer = Arc::new(SharedBuffer::new(1024));
    let handles: Vec<_> = (0..10).map(|i| {
        let buf = buffer.clone();
        thread::spawn(move || {
            // Concurrent access
            if let Some(val) = buf.get(i * 10) {
                println!("Thread {} read: {}", i, val);
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }
}