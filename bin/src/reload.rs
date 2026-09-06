// Copyright 2026 Lyam Witherow
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// https://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// https://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Runtime catalog swap so SIGHUP can reload zone files without dropping sockets.

use std::sync::Arc;

use hickory_server::{
    net::runtime::Time,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    zone_handler::Catalog,
};
use tokio::sync::RwLock;

/// Request handler that can atomically replace its [`Catalog`].
///
/// Lookups take a read lock for the duration of the request. A reload takes a
/// write lock, so in-flight queries finish against the previous catalog and new
/// queries see the replacement. Listen sockets are owned by [`Server`] and are
/// not rebound.
#[derive(Clone)]
pub(crate) struct ReloadingCatalog {
    inner: Arc<RwLock<Catalog>>,
}

impl ReloadingCatalog {
    pub(crate) fn new(catalog: Catalog) -> Self {
        Self {
            inner: Arc::new(RwLock::new(catalog)),
        }
    }

    pub(crate) async fn replace(&self, catalog: Catalog) {
        *self.inner.write().await = catalog;
    }
}

#[async_trait::async_trait]
impl RequestHandler for ReloadingCatalog {
    async fn handle_request<R: ResponseHandler, T: Time>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> ResponseInfo {
        self.inner
            .read()
            .await
            .handle_request::<R, T>(request, response_handle)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replace_swaps_catalog() {
        let catalog = ReloadingCatalog::new(Catalog::new());
        catalog.replace(Catalog::new()).await;
    }
}
