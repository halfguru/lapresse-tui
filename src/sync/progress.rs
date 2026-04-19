#[derive(Default)]
pub struct SyncStats {
    pub days_scraped: u32,
    pub days_failed: u32,
    pub articles_total: u32,
    pub images_total: u32,
    pub retries: u32,
    pub articles_blocked: u32,
}
