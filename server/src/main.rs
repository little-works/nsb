fn main() {
    let config = nsb_core::ServerConfig::default();

    println!("nsb-server");
    println!("listening on {}", config.listen_addr);
}
