use std::sync::Arc;
use uxrp::session::SessionStore;
use uxrp::user::UserStore;
use uxrp_protocol::actix_web::{self, web, App, HttpServer};
use uxrp_protocol::async_trait::async_trait;
use uxrp_protocol::auth::*;
use uxrp_protocol::core::{HttpPrincipalResolver, Result, UserPrincipal};

struct AuthService {
	session_store: Arc<SessionStore>,
	user_store: UserStore,
}

#[async_trait]
impl Service for AuthService {
	async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse> {
		self.user_store.create(&req.email, &req.password).await?;
		Ok(RegisterResponse {})
	}

	async fn login(&self, req: &LoginRequest) -> Result<LoginResponse> {
		let id = self.user_store.verify(&req.email, &req.password).await?;
		let token = self.session_store.create(UserPrincipal { id }).await?;
		Ok(LoginResponse { token })
	}

	async fn test(&self, _req: &TestRequest, caller: &UserPrincipal) -> Result<TestResponse> {
		Ok(TestResponse {
			principal_id: caller.id.clone(),
		})
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let session_store = Arc::new(
		SessionStore::new("redis://localhost:25001")
			.await
			.expect("failed to init user resolver"),
	);

	let user_store = UserStore::new();

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::from(
				session_store.clone() as Arc<dyn HttpPrincipalResolver<UserPrincipal>>
			))
			.service(create_scope(Arc::new(AuthService {
				session_store: session_store.clone(),
				user_store: user_store.clone(),
			})))
	})
	.bind(("0.0.0.0", 1337))?
	.run()
	.await
}
