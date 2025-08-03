pub async fn run() -> &'static str {
    
    tokio::process::Command::new("java");
    "ran!"
}
