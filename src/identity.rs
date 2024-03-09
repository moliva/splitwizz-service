use std::{
    cell::{Ref, RefCell},
    env,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    dev::{Extensions, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::HeaderMap,
    Error, FromRequest, HttpMessage, HttpRequest, Result,
};
use futures::{
    future::{ok, FutureExt, Ready},
    Future,
};
use google_jwt_verify::{Client as GoogleClient, IdPayload, Token};
use http::Method;

#[derive(Clone)]
pub struct Identity(HttpRequest);

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Identity, Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Identity(req.clone()))
    }
}

impl Identity {
    /// Return the claimed identity of the user associated request or
    /// ``None`` if no identity can be found associated with the request.
    pub fn identity(&self) -> Option<IdToken> {
        Identity::get_identity(&self.0.extensions())
    }

    /// Remember identity.
    pub fn remember(&self, identity: IdToken) {
        if let Some(id) = self.0.extensions_mut().get_mut::<IdentityItem>() {
            id.id = Some(identity);
        }
    }

    /// This method is used to 'forget' the current identity on subsequent
    /// requests.
    pub fn forget(&self) {
        if let Some(id) = self.0.extensions_mut().get_mut::<IdentityItem>() {
            id.id = None;
        }
    }

    fn get_identity(extensions: &Ref<'_, Extensions>) -> Option<IdToken> {
        if let Some(id) = extensions.get::<IdentityItem>() {
            id.id.clone()
        } else {
            None
        }
    }
}

struct IdentityItem {
    id: Option<IdToken>,
}

#[derive(Clone)]
pub struct IdToken {
    pub email: String,
    pub name: String,
}

impl From<Token<IdPayload>> for IdToken {
    fn from(t: Token<IdPayload>) -> Self {
        let email = t.get_payload().get_email();
        let name = t.get_payload().get_name();

        Self { email, name }
    }
}

pub struct IdentityService;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for IdentityService
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S: 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = IdentityMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(IdentityMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct IdentityMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S> Clone for IdentityMiddleware<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
        }
    }
}

impl<S, B> Service<ServiceRequest> for IdentityMiddleware<S>
where
    B: 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req.method() == Method::OPTIONS {
            return Box::pin(self.service.call(req));
        }
        if ["/login", "/auth"].contains(&req.path()) {
            return Box::pin(self.service.call(req));
        }

        let headers = req.headers().clone();
        let srv = self.service.clone();

        async move {
            let id = validate_auth_(&headers).await;
            let id = id.map(IdToken::from);
            req.extensions_mut().insert(IdentityItem { id });

            let fut = srv.borrow_mut().call(req);
            let res = fut.await?;
            res.request().extensions_mut().remove::<IdentityItem>();

            Ok(res)
        }
        .boxed_local()
    }
}

async fn validate_auth_(headers: &HeaderMap) -> Option<Token<IdPayload>> {
    let authorization = headers.get("authorization");

    if let Some(identity_token) = authorization {
        let client_id = env::var("CLIENT_ID").unwrap();
        let client_id = client_id.as_str();
        let token = identity_token.to_str().unwrap();

        let client = GoogleClient::new(client_id);
        let yas = client.verify_id_token_async(token).await.unwrap();

        Some(yas)
    } else {
        None
    }
}
