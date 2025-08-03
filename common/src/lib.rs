use bitcode::Encode;

#[derive(Debug, Default, Clone, Encode)]
pub struct Stats {
    /// usage per cpu (core).
    pub cpu_usages: Vec<f32>,
    pub ram_used: u64,
    pub ram_free: u64,
}