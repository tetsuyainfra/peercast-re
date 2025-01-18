use std::task::{Context, Poll};

use axum_client_ip::{SecureClientIp, SecureClientIpSource};
use http::{Request, Response};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct SetRemoteIpLayer {
    ip_src: SecureClientIpSource,
}

impl SetRemoteIpLayer {
    pub fn new(ip_src: SecureClientIpSource) -> Self {
        Self { ip_src }
    }
}

impl<S> Layer<S> for SetRemoteIpLayer {
    type Service = SetRemoteIp<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SetRemoteIp::new(inner, self.ip_src.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SetRemoteIp<S> {
    inner: S,
    ip_src: SecureClientIpSource,
}

impl<S> SetRemoteIp<S> {
    fn new(inner: S, ip_src: SecureClientIpSource) -> Self {
        Self { inner, ip_src }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for SetRemoteIp<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // let real_remote_ip = SecureClientIp::from(&self.ip_src, req.headers(), req.extensions());
        if let Ok(real_remote_ip) =
            SecureClientIp::from(&self.ip_src, req.headers(), req.extensions())
        {
            req.extensions_mut().insert(real_remote_ip);
        }

        // if let Some(request_id) = req.headers().get(&self.header_name) {
        //     if req.extensions().get::<RequestId>().is_none() {
        //         let request_id = request_id.clone();
        //         req.extensions_mut().insert(RequestId::new(request_id));
        //     }
        // } else if let Some(request_id) = self.make_request_id.make_request_id(&req) {
        //     req.extensions_mut().insert(request_id.clone());
        //     req.headers_mut()
        //         .insert(self.header_name.clone(), request_id.0);
        // }

        self.inner.call(req)
    }
}

#[derive(Debug)]
pub struct RemoteIp {}

#[cfg(test)]
mod t {
    #[test]
    fn test() {
        use std::any::{Any, TypeId};

        let boxed: Box<dyn Any> = Box::new(3_i32);

        // You're more likely to want this:
        let actual_id = (&*boxed).type_id();
        let boxed_id = boxed.type_id();

        println!("{:?}", &actual_id);
        println!("{:?}", &boxed_id);

        let actual_id = (&*boxed).type_id();
        println!("{:?}", &actual_id);

        assert_eq!(actual_id, TypeId::of::<i32>());
        assert_eq!(boxed_id, TypeId::of::<Box<dyn Any>>());
    }
}
