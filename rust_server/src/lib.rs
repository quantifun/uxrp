pub use actix_web;
pub use async_trait;
pub mod core {
	#[async_trait::async_trait(?Send)]
	pub trait HttpPrincipalResolver<P> {
		async fn resolve(&self, req: actix_web::HttpRequest) -> Result<P, actix_web::HttpResponse>;
	}
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub struct UserPrincipal {
		pub id: String,
	}
}
pub mod auth {
	use crate::core::*;
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	pub enum Error {
		UserExists,
		InvalidCredentials,
	}
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
	#[async_trait::async_trait]
	pub trait Service {
		async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse, Error>;
		async fn login(&self, req: &LoginRequest) -> Result<LoginResponse, Error>;
		async fn test(&self, req: &TestRequest, caller: &UserPrincipal) -> Result<TestResponse, Error>;
	}
	async fn http_handler_register(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<RegisterRequest>,
	) -> impl actix_web::Responder {
		let result = svc.register(&req).await;
		match result {
			Ok(r) => actix_web::HttpResponse::Ok().json(r),
			Err(err) => actix_web::HttpResponse::BadRequest().json(err),
		}
	}
	async fn http_handler_login(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<LoginRequest>,
	) -> impl actix_web::Responder {
		let result = svc.login(&req).await;
		match result {
			Ok(r) => actix_web::HttpResponse::Ok().json(r),
			Err(err) => actix_web::HttpResponse::BadRequest().json(err),
		}
	}
	async fn http_handler_test(
		svc: actix_web::web::Data<dyn Service>,
		req: actix_web::web::Json<TestRequest>,
		resolver: actix_web::web::Data<dyn HttpPrincipalResolver<UserPrincipal>>,
		http_req: actix_web::HttpRequest,
	) -> impl actix_web::Responder {
		let result = svc
			.test(
				&req,
				match resolver.resolve(http_req).await {
					Ok(ref p) => p,
					Err(err) => return err,
				},
			)
			.await;
		match result {
			Ok(r) => actix_web::HttpResponse::Ok().json(r),
			Err(err) => actix_web::HttpResponse::BadRequest().json(err),
		}
	}
	pub fn create_scope(svc: std::sync::Arc<dyn Service>) -> actix_web::Scope {
		actix_web::web::scope("auth")
			.app_data(actix_web::web::Data::from(svc))
			.service(actix_web::web::resource("register").route(actix_web::web::post().to(http_handler_register)))
			.service(actix_web::web::resource("login").route(actix_web::web::post().to(http_handler_login)))
			.service(actix_web::web::resource("test").route(actix_web::web::post().to(http_handler_test)))
	}
}
