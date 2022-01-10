use config::Config;
use serde::Deserialize;
use std::sync::Arc;
use uxrp::session::{SessionStore, SessionStoreConfig};
use uxrp::user::{UserStore, UserStoreConfig};
use uxrp_protocol::actix_web::{web, App, HttpServer};
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

#[derive(Deserialize)]
struct AppConfig {
	user_store: UserStoreConfig,
	session_store: SessionStoreConfig,
}

// TODO: deserialize's replacement in `config`, try_into, conflicts with the new 2021 prelude
// looks to be fixed in github latest, so replace once new crates.io version is out
#[allow(deprecated)]
fn load_conf() -> AppConfig {
	let mut config = Config::new();

	config
		.merge(config::File::with_name("settings.toml"))
		.expect("config retrieval failed");

	config.deserialize().expect("config deserialisation failed")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let config = load_conf();

	let session_store = Arc::new(
		SessionStore::new(config.session_store)
			.await
			.expect("failed to init user resolver"),
	);

	let user_store = UserStore::new(config.user_store).await;

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
