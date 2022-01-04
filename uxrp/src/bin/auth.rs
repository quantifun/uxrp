use std::sync::Arc;
use uxrp::session::UserPrincipalResolver;
use uxrp_protocol::actix_web::{self, web, App, HttpServer};
use uxrp_protocol::async_trait::async_trait;
use uxrp_protocol::auth::*;
use uxrp_protocol::core::{HttpPrincipalResolver, UserPrincipal};

struct AuthService {}

#[async_trait]
impl Service for AuthService {
	async fn register(&self, _req: &RegisterRequest) -> Result<RegisterResponse, Error> {
		todo!()
	}

	async fn login(&self, _req: &LoginRequest) -> Result<LoginResponse, Error> {
		todo!()
	}

	async fn test(&self, _req: &TestRequest, caller: &UserPrincipal) -> Result<TestResponse, Error> {
		Ok(TestResponse {
			principal_id: caller.id.clone(),
		})
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let resolver = Arc::new(
		UserPrincipalResolver::new("redis://localhost:25001")
			.await
			.expect("failed to init user resolver"),
	);

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::from(
				resolver.clone() as Arc<dyn HttpPrincipalResolver<UserPrincipal>>
			))
			.service(create_scope(Arc::new(AuthService {})))
	})
	.bind(("0.0.0.0", 1337))?
	.run()
	.await
}
