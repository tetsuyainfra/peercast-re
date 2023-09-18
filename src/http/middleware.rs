use std::{
    marker::PhantomData,
    task::{Context, Poll},
};

use axum::{body::HttpBody, extract::ConnectInfo};
use axum_core::{body::Body, extract::Request, response::Response};
use futures_util::{future::BoxFuture, Future};
use http::{HeaderValue, StatusCode};
use ipnet::IpNet;
use tower::{Layer, Service};

use crate::error;

use super::MyConnectInfo;

#[derive(Clone)]
pub struct RestrictIpLayer {
    pub white_nets: Vec<IpNet>,
}

impl<S> Layer<S> for RestrictIpLayer {
    type Service = RestrictIpMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RestrictIpMiddleware {
            inner,
            white_nets: self.white_nets.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RestrictIpMiddleware<S> {
    inner: S,
    white_nets: Vec<IpNet>,
}

impl<S> Service<Request<Body>> for RestrictIpMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let path = request.uri().path();
        let Some(info) = request.extensions().get::<ConnectInfo<MyConnectInfo>>() else {
            tracing::error!("can't use MyConnectInfo");
            return Box::pin(async move { Ok(create_error_response()) });
        };
        let ip = &info.remote.ip();
        if None == self.white_nets.iter().find(|net| net.contains(ip)) {
            tracing::trace!("IP is NOT ALLOWED, {ip}");
            return Box::pin(async move { Ok(create_error_response()) });
        };

        // normal
        tracing::trace!("IP is allowed, {ip}");
        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}

pub(crate) fn create_error_response() -> Response<Body> {
    let mut res = Response::new(Body::from("Restrict"));
    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;

    #[allow(clippy::declare_interior_mutable_const)]
    const TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain; charset=utf-8");
    res.headers_mut()
        .insert(http::header::CONTENT_TYPE, TEXT_PLAIN);

    res
}
