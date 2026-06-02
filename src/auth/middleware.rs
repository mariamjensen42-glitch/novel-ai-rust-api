use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest};
use futures_util::future::LocalBoxFuture;

use crate::auth::jwt::verify_token;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
}

impl FromRequest for CurrentUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let ext = req.extensions();
        match ext.get::<CurrentUser>().cloned() {
            Some(u) => ready(Ok(u)),
            None => ready(Err(AppError::Unauthorized.into())),
        }
    }
}

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            let path = req.path().to_string();
            if is_public_path(&path) {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            let header = req
                .headers()
                .get(actix_web::http::header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            let token = header.and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()));

            match token.and_then(|t| verify_token(&t).ok()) {
                Some(claims) => {
                    req.extensions_mut().insert(CurrentUser { id: claims.sub });
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                None => {
                    let err: Error = AppError::Unauthorized.into();
                    let (request, _pl) = req.into_parts();
                    let response = err.error_response();
                    let new_response = ServiceResponse::new(request, response).map_into_right_body();
                    Ok(new_response)
                }
            }
        })
    }
}

fn is_public_path(path: &str) -> bool {
    path == "/health"
        || path.starts_with("/auth/register")
        || path.starts_with("/auth/login")
        || path.starts_with("/docs")
        || path.starts_with("/api-docs")
}
