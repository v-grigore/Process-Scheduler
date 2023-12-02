//! Implement the schedulers in this module
//!
//! You might want to create separate files
//! for each scheduler and export it here
//! like
//!
//! ```ignore
//! mod scheduler_name
//! pub use scheduler_name::SchedulerName;
//! ```
//!
mod round_robin;
pub use round_robin::RoundRobin;

mod priority_queue;
pub use priority_queue::PriorityQueue;

mod cfs;
pub use cfs::CFS;
