use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const MAX_COMPUTE_NODES: usize = 16;
pub const MAX_PROVIDERS: usize = 32;
pub const MAX_MODELS: usize = 128;
pub const MAX_ID_LENGTH: usize = 128;
pub const MAX_MODEL_REFS_PER_PROVIDER: usize = 64;
pub const MAX_LOADED_MODELS_PER_NODE: usize = 32;
pub const MAX_GPUS_PER_NODE: usize = 8;
pub const MAX_QUEUE_CAPACITY: u16 = 64;
pub const MAX_RESERVATIONS: usize = 256;
pub const MAX_MEMORY_MB: u64 = 2_000_000;
pub const MAX_PERFORMANCE_SCORE: u32 = 1_000_000;
pub const MAX_LATENCY_MS: u32 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    TextGeneration,
    Vision,
    Embeddings,
    SpeechToText,
    TextToSpeech,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityState {
    Connected,
    Disconnected,
    Checking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HealthSnapshot {
    pub state: HealthState,
    pub connectivity: ConnectivityState,
    pub last_health_at_ms: Option<u64>,
    pub latency_ms: Option<u32>,
}

impl HealthSnapshot {
    fn validate(&self) -> Result<(), OrchestrationError> {
        if self
            .latency_ms
            .is_some_and(|latency| latency > MAX_LATENCY_MS)
        {
            return Err(OrchestrationError::InvalidContract("health_latency"));
        }
        Ok(())
    }

    fn is_usable(self) -> bool {
        matches!(self.state, HealthState::Healthy | HealthState::Degraded)
            && self.connectivity == ConnectivityState::Connected
    }

    fn score(self) -> i64 {
        match self.state {
            HealthState::Healthy => 1_000,
            HealthState::Degraded => 250,
            HealthState::Unavailable | HealthState::Unknown => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GpuResource {
    pub id: String,
    pub vram_total_mb: u64,
    pub vram_available_mb: u64,
    pub estimated_performance: u32,
}

impl GpuResource {
    fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.id, "gpu_id")?;
        if self.vram_total_mb == 0
            || self.vram_total_mb > MAX_MEMORY_MB
            || self.vram_available_mb > self.vram_total_mb
            || self.estimated_performance > MAX_PERFORMANCE_SCORE
        {
            return Err(OrchestrationError::InvalidContract("gpu_resources"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadedModel {
    pub model_ref: String,
    pub gpu_id: Option<String>,
    pub vram_mb: u64,
    pub last_used_at_ms: Option<u64>,
}

impl LoadedModel {
    fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.model_ref, "loaded_model_ref")?;
        if let Some(gpu_id) = &self.gpu_id {
            validate_id(gpu_id, "loaded_gpu_id")?;
        }
        if self.vram_mb > MAX_MEMORY_MB {
            return Err(OrchestrationError::InvalidContract("loaded_model_memory"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueLoad {
    pub depth: u16,
    pub active: u16,
    pub capacity: u16,
}

impl QueueLoad {
    fn validate(&self) -> Result<(), OrchestrationError> {
        if self.capacity == 0
            || self.capacity > MAX_QUEUE_CAPACITY
            || self.depth > self.capacity
            || self.active > self.depth
        {
            return Err(OrchestrationError::InvalidContract("queue_load"));
        }
        Ok(())
    }

    fn is_full(&self) -> bool {
        self.depth >= self.capacity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputeNode {
    pub id: String,
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub gpus: Vec<GpuResource>,
    pub queue: QueueLoad,
    pub loaded_models: Vec<LoadedModel>,
    pub priority: i32,
    pub estimated_performance: u32,
    pub estimated_latency_ms: u32,
    pub health: HealthSnapshot,
}

impl ComputeNode {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.id, "node_id")?;
        if self.ram_total_mb == 0
            || self.ram_total_mb > MAX_MEMORY_MB
            || self.ram_available_mb > self.ram_total_mb
            || self.priority.unsigned_abs() > 1_000
            || self.estimated_performance > MAX_PERFORMANCE_SCORE
            || self.estimated_latency_ms > MAX_LATENCY_MS
            || self.gpus.len() > MAX_GPUS_PER_NODE
            || self.loaded_models.len() > MAX_LOADED_MODELS_PER_NODE
        {
            return Err(OrchestrationError::InvalidContract("compute_node"));
        }
        for gpu in &self.gpus {
            gpu.validate()?;
        }
        for loaded_model in &self.loaded_models {
            loaded_model.validate()?;
        }
        self.queue.validate()?;
        self.health.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provider {
    pub id: String,
    pub node_id: String,
    pub model_refs: Vec<String>,
    pub priority: i32,
    pub health: HealthSnapshot,
}

impl Provider {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.id, "provider_id")?;
        validate_id(&self.node_id, "provider_node_id")?;
        if self.model_refs.is_empty()
            || self.model_refs.len() > MAX_MODEL_REFS_PER_PROVIDER
            || self.priority.unsigned_abs() > 1_000
        {
            return Err(OrchestrationError::InvalidContract("provider"));
        }
        for model_ref in &self.model_refs {
            validate_id(model_ref, "provider_model_ref")?;
        }
        self.health.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelSpec {
    pub model_ref: String,
    pub provider_id: String,
    pub capabilities: Vec<ModelCapability>,
    pub vram_mb: u64,
    pub ram_mb: u64,
    pub estimated_performance: u32,
    pub estimated_latency_ms: u32,
}

impl ModelSpec {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.model_ref, "model_ref")?;
        validate_id(&self.provider_id, "model_provider_id")?;
        if self.capabilities.is_empty()
            || self.capabilities.len() > 8
            || self.vram_mb > MAX_MEMORY_MB
            || self.ram_mb == 0
            || self.ram_mb > MAX_MEMORY_MB
            || self.estimated_performance > MAX_PERFORMANCE_SCORE
            || self.estimated_latency_ms > MAX_LATENCY_MS
        {
            return Err(OrchestrationError::InvalidContract("model_spec"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestPriority {
    Background,
    Maintenance,
    OwnerCommand,
    ActiveConversation,
}

impl RequestPriority {
    fn score(self) -> i64 {
        match self {
            RequestPriority::Background => 50,
            RequestPriority::Maintenance => 150,
            RequestPriority::OwnerCommand => 350,
            RequestPriority::ActiveConversation => 500,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingRequest {
    pub request_id: String,
    pub model_ref: Option<String>,
    pub capability: ModelCapability,
    pub priority: RequestPriority,
    pub additional_ram_mb: u64,
    pub max_latency_ms: Option<u32>,
    pub created_at_ms: u64,
}

impl RoutingRequest {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        validate_id(&self.request_id, "request_id")?;
        if let Some(model_ref) = &self.model_ref {
            validate_id(model_ref, "requested_model_ref")?;
        }
        if self.additional_ram_mb > MAX_MEMORY_MB
            || self
                .max_latency_ms
                .is_some_and(|latency| latency > MAX_LATENCY_MS)
        {
            return Err(OrchestrationError::InvalidContract("routing_request"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Ready,
    Queued,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RouteCandidate {
    pub node_id: String,
    pub provider_id: String,
    pub model_ref: String,
    pub score: i64,
    pub queue_status: QueueStatus,
    pub queue_position: u16,
    pub loaded_model_reuse: bool,
    pub available_vram_mb: u64,
    pub available_ram_mb: u64,
    pub estimated_latency_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationStatus {
    Reserved,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationReservation {
    pub request_id: String,
    pub node_id: String,
    pub provider_id: String,
    pub model_ref: String,
    pub score: i64,
    pub queue_status: QueueStatus,
    pub queue_position: u16,
    pub loaded_model_reuse: bool,
    pub reserved_ram_mb: u64,
    pub status: ReservationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationError {
    InvalidContract(&'static str),
    RegistryFull,
    NodeNotFound,
    ProviderNotFound,
    NoCompatibleCandidate,
    NoHealthyCandidate,
    NoResources,
    QueueFull,
    ReservationFailed,
    ReservationStoreFull,
    ReservationNotFound,
}

#[derive(Debug, Default)]
pub struct OrchestrationManager {
    nodes: BTreeMap<String, ComputeNode>,
    providers: BTreeMap<String, Provider>,
    models: BTreeMap<String, ModelSpec>,
    reservations: BTreeMap<String, GenerationReservation>,
}

impl OrchestrationManager {
    pub fn register_node(&mut self, node: ComputeNode) -> Result<(), OrchestrationError> {
        node.validate()?;
        if !self.nodes.contains_key(&node.id) && self.nodes.len() >= MAX_COMPUTE_NODES {
            return Err(OrchestrationError::RegistryFull);
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn register_provider(&mut self, provider: Provider) -> Result<(), OrchestrationError> {
        provider.validate()?;
        if !self.nodes.contains_key(&provider.node_id) {
            return Err(OrchestrationError::NodeNotFound);
        }
        if !self.providers.contains_key(&provider.id) && self.providers.len() >= MAX_PROVIDERS {
            return Err(OrchestrationError::RegistryFull);
        }
        self.providers.insert(provider.id.clone(), provider);
        Ok(())
    }

    pub fn register_model(&mut self, model: ModelSpec) -> Result<(), OrchestrationError> {
        model.validate()?;
        let provider = self
            .providers
            .get(&model.provider_id)
            .ok_or(OrchestrationError::ProviderNotFound)?;
        if !provider
            .model_refs
            .iter()
            .any(|model_ref| model_ref == &model.model_ref)
        {
            return Err(OrchestrationError::InvalidContract(
                "provider_model_mismatch",
            ));
        }
        if !self.models.contains_key(&model.model_ref) && self.models.len() >= MAX_MODELS {
            return Err(OrchestrationError::RegistryFull);
        }
        self.models.insert(model.model_ref.clone(), model);
        Ok(())
    }

    pub fn update_node_health(
        &mut self,
        node_id: &str,
        health: HealthSnapshot,
    ) -> Result<(), OrchestrationError> {
        health.validate()?;
        self.nodes
            .get_mut(node_id)
            .ok_or(OrchestrationError::NodeNotFound)?
            .health = health;
        Ok(())
    }

    pub fn update_provider_health(
        &mut self,
        provider_id: &str,
        health: HealthSnapshot,
    ) -> Result<(), OrchestrationError> {
        health.validate()?;
        self.providers
            .get_mut(provider_id)
            .ok_or(OrchestrationError::ProviderNotFound)?
            .health = health;
        Ok(())
    }

    pub fn rank_candidates(
        &self,
        request: &RoutingRequest,
    ) -> Result<Vec<RouteCandidate>, OrchestrationError> {
        request.validate()?;
        let mut compatible = false;
        let mut healthy = false;
        let mut resource_fit = false;
        let mut queue_available = false;
        let mut candidates = Vec::new();

        for provider in self.providers.values() {
            let Some(node) = self.nodes.get(&provider.node_id) else {
                continue;
            };
            for model_ref in &provider.model_refs {
                let Some(model) = self.models.get(model_ref) else {
                    continue;
                };
                if request
                    .model_ref
                    .as_ref()
                    .is_some_and(|requested| requested != model_ref)
                    || !model.capabilities.contains(&request.capability)
                {
                    continue;
                }
                compatible = true;
                let estimated_latency_ms = effective_latency(node, provider, model);
                if request
                    .max_latency_ms
                    .is_some_and(|maximum| estimated_latency_ms > maximum)
                {
                    continue;
                }
                if !node.health.is_usable() || !provider.health.is_usable() {
                    continue;
                }
                healthy = true;
                let loaded_model_reuse = node
                    .loaded_models
                    .iter()
                    .any(|loaded| loaded.model_ref == model.model_ref);
                let required_ram_mb = model.ram_mb.saturating_add(request.additional_ram_mb);
                let available_vram_mb = available_vram(node);
                if node.ram_available_mb < required_ram_mb
                    || (!loaded_model_reuse
                        && model.vram_mb > 0
                        && first_fitting_gpu(node, model.vram_mb).is_none())
                {
                    continue;
                }
                resource_fit = true;
                if node.queue.is_full() {
                    continue;
                }
                queue_available = true;
                let queue_status = if node.queue.active == 0 && node.queue.depth == 0 {
                    QueueStatus::Ready
                } else {
                    QueueStatus::Queued
                };
                candidates.push(RouteCandidate {
                    node_id: node.id.clone(),
                    provider_id: provider.id.clone(),
                    model_ref: model.model_ref.clone(),
                    score: route_score(
                        request,
                        node,
                        provider,
                        model,
                        loaded_model_reuse,
                        estimated_latency_ms,
                    ),
                    queue_status,
                    queue_position: if queue_status == QueueStatus::Ready {
                        0
                    } else {
                        node.queue.depth
                    },
                    loaded_model_reuse,
                    available_vram_mb,
                    available_ram_mb: node.ram_available_mb,
                    estimated_latency_ms,
                });
            }
        }

        if candidates.is_empty() {
            if !compatible {
                return Err(OrchestrationError::NoCompatibleCandidate);
            }
            if !healthy {
                return Err(OrchestrationError::NoHealthyCandidate);
            }
            if !resource_fit {
                return Err(OrchestrationError::NoResources);
            }
            if !queue_available {
                return Err(OrchestrationError::QueueFull);
            }
            return Err(OrchestrationError::NoResources);
        }

        candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.node_id.cmp(&right.node_id))
                .then_with(|| left.provider_id.cmp(&right.provider_id))
                .then_with(|| left.model_ref.cmp(&right.model_ref))
        });
        Ok(candidates)
    }

    pub fn reserve(
        &mut self,
        request: RoutingRequest,
    ) -> Result<GenerationReservation, OrchestrationError> {
        self.reserve_with(request, |_| Ok(()))
    }

    pub fn reserve_with<F>(
        &mut self,
        request: RoutingRequest,
        mut try_reserve: F,
    ) -> Result<GenerationReservation, OrchestrationError>
    where
        F: FnMut(&RouteCandidate) -> Result<(), &'static str>,
    {
        request.validate()?;
        if let Some(existing) = self.reservations.get(&request.request_id) {
            return Ok(existing.clone());
        }
        if self.reservations.len() >= MAX_RESERVATIONS {
            return Err(OrchestrationError::ReservationStoreFull);
        }
        let candidates = self.rank_candidates(&request)?;
        for candidate in candidates {
            if try_reserve(&candidate).is_err() {
                continue;
            }
            let reservation = self.commit_reservation(&request, &candidate)?;
            self.reservations
                .insert(request.request_id.clone(), reservation.clone());
            return Ok(reservation);
        }
        Err(OrchestrationError::ReservationFailed)
    }

    pub fn complete(
        &mut self,
        request_id: &str,
    ) -> Result<GenerationReservation, OrchestrationError> {
        let existing = self
            .reservations
            .get(request_id)
            .cloned()
            .ok_or(OrchestrationError::ReservationNotFound)?;
        if existing.status == ReservationStatus::Completed {
            return Ok(existing);
        }
        let node = self
            .nodes
            .get_mut(&existing.node_id)
            .ok_or(OrchestrationError::NodeNotFound)?;
        if node.queue.depth == 0 || node.ram_available_mb > node.ram_total_mb {
            return Err(OrchestrationError::InvalidContract("reservation_state"));
        }
        node.queue.depth -= 1;
        node.ram_available_mb = node
            .ram_available_mb
            .saturating_add(existing.reserved_ram_mb)
            .min(node.ram_total_mb);
        let mut completed = existing;
        completed.status = ReservationStatus::Completed;
        self.reservations
            .insert(request_id.to_string(), completed.clone());
        Ok(completed)
    }

    pub fn queue_status(&self, node_id: &str) -> Result<QueueLoad, OrchestrationError> {
        self.nodes
            .get(node_id)
            .map(|node| node.queue.clone())
            .ok_or(OrchestrationError::NodeNotFound)
    }

    fn commit_reservation(
        &mut self,
        request: &RoutingRequest,
        candidate: &RouteCandidate,
    ) -> Result<GenerationReservation, OrchestrationError> {
        let model = self
            .models
            .get(&candidate.model_ref)
            .cloned()
            .ok_or(OrchestrationError::NoCompatibleCandidate)?;
        let node = self
            .nodes
            .get_mut(&candidate.node_id)
            .ok_or(OrchestrationError::NodeNotFound)?;
        let required_ram_mb = model.ram_mb.saturating_add(request.additional_ram_mb);
        if node.queue.is_full() || node.ram_available_mb < required_ram_mb {
            return Err(OrchestrationError::NoResources);
        }
        let loaded_model_reuse = node
            .loaded_models
            .iter()
            .any(|loaded| loaded.model_ref == model.model_ref);
        let gpu_id = if loaded_model_reuse || model.vram_mb == 0 {
            None
        } else {
            let index =
                first_fitting_gpu(node, model.vram_mb).ok_or(OrchestrationError::NoResources)?;
            let gpu = node
                .gpus
                .get_mut(index)
                .ok_or(OrchestrationError::NoResources)?;
            gpu.vram_available_mb -= model.vram_mb;
            Some(gpu.id.clone())
        };
        if !loaded_model_reuse {
            node.loaded_models.push(LoadedModel {
                model_ref: model.model_ref.clone(),
                gpu_id,
                vram_mb: model.vram_mb,
                last_used_at_ms: Some(request.created_at_ms),
            });
        }
        node.ram_available_mb -= required_ram_mb;
        let queue_status = if node.queue.active == 0 && node.queue.depth == 0 {
            QueueStatus::Ready
        } else {
            QueueStatus::Queued
        };
        let queue_position = if queue_status == QueueStatus::Ready {
            0
        } else {
            node.queue.depth
        };
        node.queue.depth += 1;
        Ok(GenerationReservation {
            request_id: request.request_id.clone(),
            node_id: candidate.node_id.clone(),
            provider_id: candidate.provider_id.clone(),
            model_ref: candidate.model_ref.clone(),
            score: candidate.score,
            queue_status,
            queue_position,
            loaded_model_reuse,
            reserved_ram_mb: required_ram_mb,
            status: ReservationStatus::Reserved,
        })
    }
}

fn validate_id(value: &str, field: &'static str) -> Result<(), OrchestrationError> {
    if value.is_empty()
        || value.len() > MAX_ID_LENGTH
        || value.chars().any(|character| character.is_control())
    {
        return Err(OrchestrationError::InvalidContract(field));
    }
    Ok(())
}

fn available_vram(node: &ComputeNode) -> u64 {
    node.gpus
        .iter()
        .map(|gpu| gpu.vram_available_mb)
        .max()
        .unwrap_or(0)
}

fn first_fitting_gpu(node: &ComputeNode, vram_mb: u64) -> Option<usize> {
    node.gpus
        .iter()
        .position(|gpu| gpu.vram_available_mb >= vram_mb)
}

fn effective_latency(node: &ComputeNode, provider: &Provider, model: &ModelSpec) -> u32 {
    node.estimated_latency_ms
        .saturating_add(provider.health.latency_ms.unwrap_or(0))
        .saturating_add(node.health.latency_ms.unwrap_or(0))
        .saturating_add(model.estimated_latency_ms)
}

fn route_score(
    request: &RoutingRequest,
    node: &ComputeNode,
    provider: &Provider,
    model: &ModelSpec,
    loaded_model_reuse: bool,
    estimated_latency_ms: u32,
) -> i64 {
    let hardware_performance = node
        .gpus
        .iter()
        .map(|gpu| gpu.estimated_performance)
        .max()
        .unwrap_or(0)
        .max(node.estimated_performance)
        .min(1_000) as i64;
    let model_performance = model.estimated_performance.min(1_000) as i64;
    let available_ram_ratio =
        (node.ram_available_mb.saturating_mul(100) / node.ram_total_mb.max(1)) as i64;
    let available_vram_ratio = if model.vram_mb == 0 {
        100
    } else {
        (available_vram(node).saturating_mul(100) / model.vram_mb.max(1)).min(100) as i64
    };
    let queue_penalty = i64::from(node.queue.depth) * 700 + i64::from(node.queue.active) * 1_000;
    let loaded_bonus = if loaded_model_reuse { 1_200 } else { 0 };
    let exact_model_bonus = if request.model_ref.is_some() { 300 } else { 0 };
    request.priority.score()
        + node.priority as i64 * 20
        + provider.priority as i64 * 20
        + node.health.score()
        + provider.health.score()
        + hardware_performance
        + model_performance / 4
        + available_ram_ratio
        + available_vram_ratio
        + loaded_bonus
        + exact_model_bonus
        - queue_penalty
        - i64::from(estimated_latency_ms) * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> HealthSnapshot {
        HealthSnapshot {
            state: HealthState::Healthy,
            connectivity: ConnectivityState::Connected,
            last_health_at_ms: Some(100),
            latency_ms: Some(5),
        }
    }

    fn node(id: &str, performance: u32, health: HealthSnapshot, queue: QueueLoad) -> ComputeNode {
        ComputeNode {
            id: id.into(),
            ram_total_mb: 32_000,
            ram_available_mb: 24_000,
            gpus: vec![GpuResource {
                id: format!("{id}-gpu"),
                vram_total_mb: 8_000,
                vram_available_mb: 8_000,
                estimated_performance: performance,
            }],
            queue,
            loaded_models: Vec::new(),
            priority: 0,
            estimated_performance: performance,
            estimated_latency_ms: 5,
            health,
        }
    }

    fn provider(id: &str, node_id: &str) -> Provider {
        Provider {
            id: id.into(),
            node_id: node_id.into(),
            model_refs: vec!["ollama:test".into()],
            priority: 0,
            health: healthy(),
        }
    }

    fn model() -> ModelSpec {
        ModelSpec {
            model_ref: "ollama:test".into(),
            provider_id: "provider".into(),
            capabilities: vec![ModelCapability::TextGeneration],
            vram_mb: 4_000,
            ram_mb: 4_000,
            estimated_performance: 500,
            estimated_latency_ms: 10,
        }
    }

    fn request(id: &str) -> RoutingRequest {
        RoutingRequest {
            request_id: id.into(),
            model_ref: None,
            capability: ModelCapability::TextGeneration,
            priority: RequestPriority::ActiveConversation,
            additional_ram_mb: 0,
            max_latency_ms: None,
            created_at_ms: 200,
        }
    }

    fn manager(nodes: Vec<ComputeNode>) -> OrchestrationManager {
        let mut manager = OrchestrationManager::default();
        for node in nodes {
            manager.register_node(node).unwrap();
        }
        manager
    }

    #[test]
    fn stronger_unavailable_node_loses_to_weaker_healthy_node() {
        let mut manager = manager(vec![
            node(
                "strong",
                1_000,
                HealthSnapshot {
                    state: HealthState::Unavailable,
                    ..healthy()
                },
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
            node(
                "weak",
                100,
                healthy(),
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
        ]);
        manager
            .register_provider(provider("strong-provider", "strong"))
            .unwrap();
        manager
            .register_provider(provider("provider", "weak"))
            .unwrap();
        manager.register_model(model()).unwrap();

        let reservation = manager.reserve(request("unavailable-fallback")).unwrap();
        assert_eq!(reservation.node_id, "weak");
    }

    #[test]
    fn ready_weaker_node_beats_stronger_busy_node() {
        let mut manager = manager(vec![
            node(
                "strong-busy",
                1_000,
                healthy(),
                QueueLoad {
                    depth: 1,
                    active: 1,
                    capacity: 4,
                },
            ),
            node(
                "weak-ready",
                100,
                healthy(),
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
        ]);
        manager
            .register_provider(provider("strong-provider", "strong-busy"))
            .unwrap();
        manager
            .register_provider(provider("provider", "weak-ready"))
            .unwrap();
        manager.register_model(model()).unwrap();

        let reservation = manager.reserve(request("busy-fallback")).unwrap();
        assert_eq!(reservation.node_id, "weak-ready");
        assert_eq!(reservation.queue_status, QueueStatus::Ready);
    }

    #[test]
    fn loaded_model_reuse_can_beat_stronger_unloaded_hardware() {
        let mut loaded = node(
            "loaded",
            100,
            healthy(),
            QueueLoad {
                depth: 0,
                active: 0,
                capacity: 4,
            },
        );
        loaded.loaded_models.push(LoadedModel {
            model_ref: "ollama:test".into(),
            gpu_id: Some("loaded-gpu".into()),
            vram_mb: 4_000,
            last_used_at_ms: Some(150),
        });
        let mut manager = manager(vec![
            node(
                "strong-unloaded",
                1_000,
                healthy(),
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
            loaded,
        ]);
        manager
            .register_provider(provider("strong-provider", "strong-unloaded"))
            .unwrap();
        manager
            .register_provider(provider("provider", "loaded"))
            .unwrap();
        manager.register_model(model()).unwrap();

        let reservation = manager.reserve(request("reuse-preference")).unwrap();
        assert_eq!(reservation.node_id, "loaded");
        assert!(reservation.loaded_model_reuse);
    }

    #[test]
    fn no_compatible_candidate_is_reported() {
        let mut manager = manager(vec![node(
            "text-only",
            100,
            healthy(),
            QueueLoad {
                depth: 0,
                active: 0,
                capacity: 4,
            },
        )]);
        manager
            .register_provider(provider("provider", "text-only"))
            .unwrap();
        manager.register_model(model()).unwrap();
        let mut request = request("no-vision");
        request.capability = ModelCapability::Vision;

        assert_eq!(
            manager.reserve(request),
            Err(OrchestrationError::NoCompatibleCandidate)
        );
    }

    #[test]
    fn queue_reports_ready_full_and_completed_states() {
        let mut manager = manager(vec![node(
            "queue-node",
            100,
            healthy(),
            QueueLoad {
                depth: 0,
                active: 0,
                capacity: 1,
            },
        )]);
        manager
            .register_provider(provider("provider", "queue-node"))
            .unwrap();
        manager.register_model(model()).unwrap();

        let reservation = manager.reserve(request("queue-one")).unwrap();
        assert_eq!(reservation.queue_status, QueueStatus::Ready);
        assert_eq!(reservation.queue_position, 0);
        assert_eq!(manager.queue_status("queue-node").unwrap().depth, 1);
        assert_eq!(
            manager.reserve(request("queue-two")),
            Err(OrchestrationError::QueueFull)
        );
        let completed = manager.complete("queue-one").unwrap();
        assert_eq!(completed.status, ReservationStatus::Completed);
        assert_eq!(manager.queue_status("queue-node").unwrap().depth, 0);
    }

    #[test]
    fn failed_reservation_falls_back_without_leaking_the_first_candidate() {
        let mut manager = manager(vec![
            node(
                "first",
                1_000,
                healthy(),
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
            node(
                "second",
                100,
                healthy(),
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            ),
        ]);
        manager
            .register_provider(provider("first-provider", "first"))
            .unwrap();
        manager
            .register_provider(provider("provider", "second"))
            .unwrap();
        manager.register_model(model()).unwrap();
        let mut attempts = Vec::new();

        let reservation = manager
            .reserve_with(request("reservation-fallback"), |candidate| {
                attempts.push(candidate.node_id.clone());
                if candidate.node_id == "first" {
                    Err("provider_busy")
                } else {
                    Ok(())
                }
            })
            .unwrap();
        assert_eq!(attempts, vec!["first", "second"]);
        assert_eq!(reservation.node_id, "second");
        assert_eq!(manager.queue_status("first").unwrap().depth, 0);
        assert_eq!(manager.queue_status("second").unwrap().depth, 1);
    }

    #[test]
    fn duplicate_request_returns_the_same_reservation_without_duplicate_queue_entry() {
        let mut manager = manager(vec![node(
            "idempotent",
            100,
            healthy(),
            QueueLoad {
                depth: 0,
                active: 0,
                capacity: 4,
            },
        )]);
        manager
            .register_provider(provider("provider", "idempotent"))
            .unwrap();
        manager.register_model(model()).unwrap();
        let first = manager.reserve(request("duplicate")).unwrap();
        let second = manager.reserve(request("duplicate")).unwrap();

        assert_eq!(first, second);
        assert_eq!(manager.queue_status("idempotent").unwrap().depth, 1);
    }

    #[test]
    fn contracts_validate_bounds_and_preserve_health_timestamp_serialization() {
        let health = healthy();
        assert_eq!(health.last_health_at_ms, Some(100));
        let serialized = serde_json::to_value(&health).unwrap();
        assert_eq!(serialized["lastHealthAtMs"], 100);

        let invalid = ComputeNode {
            id: "node\ninvalid".into(),
            ..node(
                "valid",
                100,
                health,
                QueueLoad {
                    depth: 0,
                    active: 0,
                    capacity: 4,
                },
            )
        };
        assert_eq!(
            invalid.validate(),
            Err(OrchestrationError::InvalidContract("node_id"))
        );
    }
}
