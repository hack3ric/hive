pub mod permission;
pub mod service;
pub mod source;

mod config;
mod error;
mod lua;
mod path;
mod task;
mod util;

pub use config::Config;
pub use error::{Error, ErrorKind, Result};
pub use lua::http::{LuaRequest, LuaResponse};
pub use mlua::Error as LuaError;
pub use service::{RunningService, RunningServiceGuard, ServiceImpl};

use hyper::{Body, Request, Response};
use lua::Sandbox;
use service::{ErrorPayload, Service, ServiceName, ServicePool, StoppedService};
use source::DirSource;
use std::path::PathBuf;
use std::sync::Arc;
use task::SandboxPool;
use uuid::Uuid;

pub struct Hive {
  sandbox_pool: SandboxPool,
  service_pool: ServicePool,
  state: Arc<HiveState>,
}

#[derive(Debug)]
pub struct HiveState {
  pub local_storage_path: PathBuf,
}

pub struct HiveOptions {
  pub sandbox_pool_size: usize,
  pub local_storage_path: PathBuf,
}

impl Hive {
  pub fn new(options: HiveOptions) -> Result<Self> {
    let state = Arc::new(HiveState {
      local_storage_path: options.local_storage_path,
    });
    let state2 = state.clone();
    Ok(Self {
      sandbox_pool: SandboxPool::new(
        "hive-worker".to_string(),
        options.sandbox_pool_size,
        move || Sandbox::new(state2.clone()),
      )?,
      service_pool: ServicePool::new(),
      state,
    })
  }

  pub async fn load_service(
    &self,
    name: impl Into<ServiceName>,
    uuid: Option<Uuid>,
    source: DirSource,
    config: Config,
  ) -> Result<(StoppedService<'_>, Option<ServiceImpl>, ErrorPayload)> {
    (self.service_pool)
      .load(&self.sandbox_pool, name.into(), uuid, source, config)
      .await
  }

  pub async fn cold_update_or_create_service(
    &self,
    name: impl Into<ServiceName>,
    uuid: Option<Uuid>,
    source: DirSource,
    config: Config,
  ) -> Result<(Service<'_>, Option<ServiceImpl>, ErrorPayload)> {
    (self.service_pool)
      .cold_update_or_create(&self.sandbox_pool, name.into(), uuid, source, config)
      .await
  }

  pub async fn hot_update_service(
    &self,
    name: impl Into<ServiceName>,
    uuid: Option<Uuid>,
    source: DirSource,
    config: Config,
  ) -> Result<(RunningService, ServiceImpl)> {
    (self.service_pool)
      .hot_update(&self.sandbox_pool, name.into(), uuid, source, config)
      .await
  }

  pub async fn preload_service(
    &self,
    name: impl Into<ServiceName>,
    uuid: Uuid,
    source: DirSource,
    config: Config,
  ) -> Result<(StoppedService<'_>, ErrorPayload)> {
    let (service, replaced, error_payload) = (self.service_pool)
      .load(&self.sandbox_pool, name.into(), Some(uuid), source, config)
      .await?;
    assert!(replaced.is_none());
    Ok((service, error_payload))
  }

  pub fn get_service(&self, name: &str) -> Result<Service<'_>> {
    (self.service_pool)
      .get(name)
      .ok_or_else(|| ErrorKind::ServiceNotFound { name: name.into() }.into())
  }

  pub fn get_running_service(&self, name: &str) -> Result<RunningService> {
    (self.service_pool)
      .get_running(name)
      .ok_or_else(|| ErrorKind::ServiceNotFound { name: name.into() }.into())
  }

  pub async fn run_service(
    &self,
    service: RunningService,
    path: String,
    req: Request<Body>,
  ) -> Result<Response<Body>> {
    (self.sandbox_pool)
      .scope(
        move |sandbox| async move { Ok(sandbox.handle_request(service, &path, req).await?.into()) },
      )
      .await
  }

  pub fn list_services(&self) -> impl Iterator<Item = Service<'_>> {
    self.service_pool.list()
  }

  pub async fn stop_service(&self, name: &str) -> Result<StoppedService<'_>> {
    self.service_pool.stop(&self.sandbox_pool, name).await
  }

  pub async fn stop_all_services(&self) {
    self.service_pool.stop_all(&self.sandbox_pool).await
  }

  pub async fn start_service(&self, name: &str) -> Result<RunningService> {
    self.service_pool.start(&self.sandbox_pool, name).await
  }

  pub async fn remove_service(&self, name: &str) -> Result<ServiceImpl> {
    self.service_pool.remove(&self.state, name).await
  }
}
