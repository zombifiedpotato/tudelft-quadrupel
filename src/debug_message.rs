use alloc::collections::vec_deque::VecDeque;

use crate::mutex::Mutex;


static DEBUG_QUEUE: Mutex<VecDeque<[u8; 32]>> = Mutex::new(VecDeque::new());


// Function to enqueue a debug message
pub fn enqueue_debug_message(message: [u8; 32]) {
    DEBUG_QUEUE.modify(|queue| {
        queue.push_back(message);
    });
}

// Function to dequeue a debug message
pub fn dequeue_debug_message() -> Option<[u8; 32]> {
    DEBUG_QUEUE.modify(|queue| {
        queue.pop_front()
    })
}

// Function to check if the queue is empty
pub fn is_debug_queue_empty() -> bool {
    DEBUG_QUEUE.modify(|queue| {
        queue.is_empty()
    })
}

pub fn debug_message_from_str(s: &str) -> [u8; 32] {
    let mut message = [0u8; 32];
    let bytes = s.as_bytes();
    
    message[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
    message
}