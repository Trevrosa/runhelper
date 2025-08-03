use bitcode::Encode;

#[derive(Debug, Default, Clone, Encode)]
pub struct Stats { 
    /// usage per cpu core.
    pub system_cpu_usage: Vec<f32>,
    // percentage use.
    pub server_cpu_usage: Option<f32>,
    // in bytes.
    pub server_ram_usage: Option<u64>,
    // bytes written + read since last refresh.
    pub server_disk_usage: Option<u64>,
    // in bytes.
    pub system_ram_used: u64,
    // in bytes.
    pub system_ram_free: u64,
}