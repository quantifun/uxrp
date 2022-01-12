use uxrp::server::get_app_config;
use uxrp_protocol::actix_web::{App, HttpServer};

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let app_config = get_app_config().await;
	HttpServer::new(move || App::new().configure(app_config.clone()))
		.bind(("0.0.0.0", 1337))?
		.run()
		.await
}
