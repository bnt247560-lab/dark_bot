use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ProgressCallback = Arc<
    dyn Fn(u8, String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
>;

pub fn scale_progress(value: u8, from: u8, to: u8) -> u8 {
    let value = value.min(100) as u16;
    let from = from as u16;
    let to = to as u16;
    (from + ((to.saturating_sub(from) * value) / 100)) as u8
}
