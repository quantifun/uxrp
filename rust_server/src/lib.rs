pub use actix_web;
pub use async_trait;
pub mod core {
	#[derive(Debug, derive_more::Display)]
	pub enum Error {
		#[display(fmt = "user_exists")]
		UserExists,
		#[display(fmt = "invalid_credentials")]
		InvalidCredentials,
		#[display(fmt = "user_unverified")]
		UserUnverified,
		#[cfg(debug_assertions)]
		#[display(fmt = "internal_error: {}", "_0")]
		Internal(Box<dyn std::error::Error>),
		#[cfg(not(debug_assertions))]
		#[display(fmt = "internal_error")]
		Internal(Box<dyn std::error::Error>),
	}
	pub type Result<T> = std::result::Result<T, Error>;
	impl<T: std::error::Error + 'static> From<T> for Error {
		fn from(err: T) -> Self {
			Error::Internal(Box::new(err))
		}
	}
	impl actix_web::error::ResponseError for Error {
		fn error_response(&self) -> actix_web::HttpResponse {
			actix_web::HttpResponseBuilder::new(self.status_code()).body(self.to_string())
		}
		fn status_code(&self) -> actix_web::http::StatusCode {
			match *self {
				Self::UserExists => actix_web::http::StatusCode::from_u16(409).unwrap(),
				Self::InvalidCredentials => actix_web::http::StatusCode::from_u16(401).unwrap(),
				Self::UserUnverified => actix_web::http::StatusCode::from_u16(403).unwrap(),
				_ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
			}
		}
	}
	#[async_trait::async_trait(?Send)]
	pub trait HttpPrincipalResolver<P> {
		async fn resolve(&self, req: actix_web::HttpRequest) -> Result<P>;
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct UserPrincipal {
		pub id: String,
	}
}
pub mod auth {
	use crate::core::*;
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct RegisterRequest {
		pub email: String,
		pub password: String,
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct RegisterResponse {}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct LoginRequest {
		pub email: String,
		pub password: String,
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct LoginResponse {
		pub token: String,
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct TestRequest {}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct TestResponse {
		pub principal_id: String,
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct VerifyRequest {
		pub token: String,
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct VerifyResponse {}
	#[async_trait::async_trait]
	pub trait Service {
		async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse>;
		async fn login(&self, req: &LoginRequest) -> Result<LoginResponse>;
		async fn test(&self, req: &TestRequest, caller: &UserPrincipal) -> Result<TestResponse>;
		async fn verify(&self, req: &VerifyRequest) -> Result<VerifyResponse>;
	}
	async fn http_handler_register(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<RegisterRequest>,
	) -> Result<actix_web::HttpResponse> {
		let result = svc.register(&req).await?;
		Ok(actix_web::HttpResponse::Ok().json(result))
	}
	async fn http_handler_login(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<LoginRequest>,
	) -> Result<actix_web::HttpResponse> {
		let result = svc.login(&req).await?;
		Ok(actix_web::HttpResponse::Ok().json(result))
	}
	async fn http_handler_test(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<TestRequest>,
		resolver: actix_web::web::Data<dyn HttpPrincipalResolver<UserPrincipal>>,
		http_req: actix_web::HttpRequest,
	) -> Result<actix_web::HttpResponse> {
		let result = svc.test(&req, &resolver.resolve(http_req).await?).await?;
		Ok(actix_web::HttpResponse::Ok().json(result))
	}
	async fn http_handler_verify(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<VerifyRequest>,
	) -> Result<actix_web::HttpResponse> {
		let result = svc.verify(&req).await?;
		Ok(actix_web::HttpResponse::Ok().json(result))
	}
	pub fn create_scope(svc: std::sync::Arc<dyn Service>) -> actix_web::Scope {
		actix_web::web::scope("auth")
			.app_data(actix_web::web::Data::from(svc))
			.service(actix_web::web::resource("register").route(actix_web::web::post().to(http_handler_register)))
			.service(actix_web::web::resource("login").route(actix_web::web::post().to(http_handler_login)))
			.service(actix_web::web::resource("test").route(actix_web::web::post().to(http_handler_test)))
			.service(actix_web::web::resource("verify").route(actix_web::web::post().to(http_handler_verify)))
	}
}
