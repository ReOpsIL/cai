use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};

/// Event types that can be published through the pub/sub system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Task-related events
    TaskCreated { task_id: String, description: String },
    TaskStarted { task_id: String, agent_type: String },
    TaskCompleted { task_id: String, success: bool, execution_time_ms: u64 },
    TaskFailed { task_id: String, error: String },
    
    /// Session-related events
    SessionCreated { session_id: String, title: String },
    SessionUpdated { session_id: String, changes: HashMap<String, serde_json::Value> },
    SessionEnded { session_id: String, duration_ms: u64 },
    
    /// Agent-related events
    AgentRegistered { agent_type: String, capabilities: Vec<String> },
    AgentStatusChanged { agent_type: String, status: AgentStatus },
    
    /// Tool-related events
    ToolExecuted { tool_name: String, success: bool, execution_time_ms: u64 },
    ToolRegistered { tool_name: String, category: String },
    
    /// Workflow events
    WorkflowStarted { workflow_id: String, goal: String },
    WorkflowCompleted { workflow_id: String, success: bool },
    GoalCreated { goal_id: String, description: String, parent_id: Option<String> },
    GoalUpdated { goal_id: String, status: String },
    
    /// Safety events
    SafetyViolation { tool_name: String, reason: String },
    PermissionRequested { tool_name: String, user_id: Option<String> },
    PermissionGranted { tool_name: String, level: String },
    
    /// System events
    SystemStarted,
    SystemShutdown,
    HealthCheck { component: String, status: HealthStatus },
    
    /// Custom events
    Custom { event_type: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Idle,
    Busy,
    Error,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Event envelope containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub event: Event,
    pub correlation_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EventEnvelope {
    pub fn new(event: Event, source: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            source,
            event,
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Trait for components that can handle events
pub trait EventSubscriber: Send + Sync {
    fn handle_event(&self, envelope: &EventEnvelope) -> Result<()>;
    fn interested_events(&self) -> Vec<EventPattern>;
}

/// Pattern for matching events
pub enum EventPattern {
    /// Match all events
    All,
    /// Match events by type
    ByType(std::mem::Discriminant<Event>),
    /// Match events by source
    BySource(String),
    /// Match events by correlation ID
    ByCorrelationId(String),
    /// Custom predicate
    Custom(Box<dyn Fn(&EventEnvelope) -> bool + Send + Sync>),
}

impl std::fmt::Debug for EventPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventPattern::All => write!(f, "All"),
            EventPattern::ByType(discriminant) => write!(f, "ByType({:?})", discriminant),
            EventPattern::BySource(source) => write!(f, "BySource({})", source),
            EventPattern::ByCorrelationId(id) => write!(f, "ByCorrelationId({})", id),
            EventPattern::Custom(_) => write!(f, "Custom(function)"),
        }
    }
}

/// Pub/Sub broker for managing event subscriptions and publishing
pub struct EventBroker {
    /// Broadcast sender for events
    sender: broadcast::Sender<EventEnvelope>,
    /// Subscribers with their patterns
    subscribers: Arc<RwLock<HashMap<String, (Weak<dyn EventSubscriber>, Vec<EventPattern>)>>>,
    /// Event statistics
    stats: Arc<RwLock<EventStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct EventStats {
    pub events_published: u64,
    pub events_delivered: u64,
    pub active_subscribers: usize,
    pub failed_deliveries: u64,
}

impl EventBroker {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer up to 1000 events
        
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(EventStats::default())),
        }
    }
    
    /// Subscribe to events with a given pattern
    pub async fn subscribe(
        &self,
        subscriber_id: String,
        subscriber: Weak<dyn EventSubscriber>,
    ) -> Result<()> {
        let patterns = match subscriber.upgrade() {
            Some(sub) => sub.interested_events(),
            None => {
                log_warn!("pub_sub", "Subscriber {} is already dropped, cannot subscribe", subscriber_id);
                return Ok(());
            }
        };
        
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscriber_id.clone(), (subscriber, patterns));
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_subscribers = subscribers.len();
        }
        
        log_info!("pub_sub", "Subscriber {} registered", subscriber_id);
        Ok(())
    }
    
    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscriber_id: &str) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(subscriber_id);
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_subscribers = subscribers.len();
        }
        
        log_info!("pub_sub", "Subscriber {} unregistered", subscriber_id);
        Ok(())
    }
    
    /// Publish an event
    pub async fn publish(&self, envelope: EventEnvelope) -> Result<()> {
        log_debug!("pub_sub", "Publishing event: {} from {}", envelope.id, envelope.source);
        
        // Send to broadcast channel
        if let Err(_) = self.sender.send(envelope.clone()) {
            log_warn!("pub_sub", "No receivers for event {}", envelope.id);
        }
        
        // Deliver to interested subscribers
        let delivered = self.deliver_to_subscribers(&envelope).await;
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.events_published += 1;
            stats.events_delivered += delivered;
        }
        
        log_debug!("pub_sub", "Event {} delivered to {} subscribers", envelope.id, delivered);
        Ok(())
    }
    
    /// Publish an event from a source
    pub async fn publish_event(&self, event: Event, source: String) -> Result<()> {
        let envelope = EventEnvelope::new(event, source);
        self.publish(envelope).await
    }
    
    /// Get a receiver for the broadcast channel
    pub fn subscribe_to_channel(&self) -> broadcast::Receiver<EventEnvelope> {
        self.sender.subscribe()
    }
    
    /// Get event statistics
    pub async fn get_stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }
    
    /// Clean up dropped subscribers
    pub async fn cleanup_subscribers(&self) {
        let mut subscribers = self.subscribers.write().await;
        let initial_count = subscribers.len();
        
        // Remove subscribers that have been dropped
        subscribers.retain(|id, (weak_sub, _)| {
            let retain = weak_sub.strong_count() > 0;
            if !retain {
                log_debug!("pub_sub", "Cleaning up dropped subscriber: {}", id);
            }
            retain
        });
        
        let cleaned = initial_count - subscribers.len();
        if cleaned > 0 {
            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_subscribers = subscribers.len();
            
            log_info!("pub_sub", "Cleaned up {} dropped subscribers", cleaned);
        }
    }
    
    /// Deliver event to interested subscribers
    async fn deliver_to_subscribers(&self, envelope: &EventEnvelope) -> u64 {
        let subscribers = self.subscribers.read().await;
        let mut delivered = 0u64;
        
        for (subscriber_id, (weak_subscriber, patterns)) in subscribers.iter() {
            if let Some(subscriber) = weak_subscriber.upgrade() {
                // Check if subscriber is interested in this event
                if self.matches_patterns(&envelope, patterns) {
                    match subscriber.handle_event(envelope) {
                        Ok(_) => {
                            delivered += 1;
                            log_debug!("pub_sub", "Event {} delivered to subscriber {}", envelope.id, subscriber_id);
                        }
                        Err(e) => {
                            log_warn!("pub_sub", "Failed to deliver event {} to subscriber {}: {}", 
                                     envelope.id, subscriber_id, e);
                            // Update failed delivery stats
                            tokio::spawn(async {
                                // Note: This is a simplified approach; in production you might want
                                // a more sophisticated error handling mechanism
                            });
                        }
                    }
                }
            }
        }
        
        delivered
    }
    
    /// Check if event matches any of the given patterns
    fn matches_patterns(&self, envelope: &EventEnvelope, patterns: &[EventPattern]) -> bool {
        patterns.iter().any(|pattern| self.matches_pattern(envelope, pattern))
    }
    
    /// Check if event matches a specific pattern
    fn matches_pattern(&self, envelope: &EventEnvelope, pattern: &EventPattern) -> bool {
        match pattern {
            EventPattern::All => true,
            EventPattern::ByType(discriminant) => {
                std::mem::discriminant(&envelope.event) == *discriminant
            }
            EventPattern::BySource(source) => envelope.source == *source,
            EventPattern::ByCorrelationId(correlation_id) => {
                envelope.correlation_id.as_ref() == Some(correlation_id)
            }
            EventPattern::Custom(predicate) => predicate(envelope),
        }
    }
}

/// Helper trait for creating event patterns
pub trait EventPatterns {
    fn task_events() -> EventPattern {
        EventPattern::Custom(Box::new(|envelope| {
            matches!(envelope.event, 
                Event::TaskCreated { .. } | 
                Event::TaskStarted { .. } | 
                Event::TaskCompleted { .. } | 
                Event::TaskFailed { .. }
            )
        }))
    }
    
    fn session_events() -> EventPattern {
        EventPattern::Custom(Box::new(|envelope| {
            matches!(envelope.event,
                Event::SessionCreated { .. } |
                Event::SessionUpdated { .. } |
                Event::SessionEnded { .. }
            )
        }))
    }
    
    fn safety_events() -> EventPattern {
        EventPattern::Custom(Box::new(|envelope| {
            matches!(envelope.event,
                Event::SafetyViolation { .. } |
                Event::PermissionRequested { .. } |
                Event::PermissionGranted { .. }
            )
        }))
    }
    
    fn system_events() -> EventPattern {
        EventPattern::Custom(Box::new(|envelope| {
            matches!(envelope.event,
                Event::SystemStarted |
                Event::SystemShutdown |
                Event::HealthCheck { .. }
            )
        }))
    }
}

impl EventPatterns for () {}

/// Example event subscriber for logging
pub struct LoggingSubscriber {
    component_name: String,
}

impl LoggingSubscriber {
    pub fn new(component_name: String) -> Self {
        Self { component_name }
    }
}

impl EventSubscriber for LoggingSubscriber {
    fn handle_event(&self, envelope: &EventEnvelope) -> Result<()> {
        match &envelope.event {
            Event::TaskCreated { task_id, description } => {
                log_info!(&self.component_name, "📝 Task created: {} - {}", task_id, description);
            }
            Event::TaskCompleted { task_id, success, execution_time_ms } => {
                let icon = if *success { "✅" } else { "❌" };
                log_info!(&self.component_name, "{} Task {} completed in {}ms", icon, task_id, execution_time_ms);
            }
            Event::SafetyViolation { tool_name, reason } => {
                log_warn!(&self.component_name, "🛡️ Safety violation: {} - {}", tool_name, reason);
            }
            Event::SystemStarted => {
                log_info!(&self.component_name, "🚀 System started");
            }
            Event::SystemShutdown => {
                log_info!(&self.component_name, "🛑 System shutdown");
            }
            _ => {
                log_debug!(&self.component_name, "📡 Event: {}", 
                          serde_json::to_string(&envelope.event).unwrap_or_else(|_| "unknown".to_string()));
            }
        }
        
        Ok(())
    }
    
    fn interested_events(&self) -> Vec<EventPattern> {
        vec![EventPattern::All]
    }
}

/// Global event broker instance
use std::sync::OnceLock;
static GLOBAL_EVENT_BROKER: OnceLock<EventBroker> = OnceLock::new();

/// Get the global event broker instance
pub fn get_global_event_broker() -> &'static EventBroker {
    GLOBAL_EVENT_BROKER.get_or_init(|| {
        log_info!("pub_sub", "Initializing global event broker");
        EventBroker::new()
    })
}

/// Initialize the global event broker
pub fn init_global_event_broker() -> &'static EventBroker {
    get_global_event_broker()
}

/// Convenience macro for publishing events
#[macro_export]
macro_rules! publish_event {
    ($event:expr, $source:expr) => {
        {
            use $crate::pub_sub::get_global_event_broker;
            let broker = get_global_event_broker();
            if let Err(e) = broker.publish_event($event, $source.to_string()).await {
                $crate::logger::log_warn!("pub_sub", "Failed to publish event: {}", e);
            }
        }
    };
}

/// Convenience function for publishing task events
pub async fn publish_task_created(task_id: String, description: String, source: String) {
    let broker = get_global_event_broker();
    let event = Event::TaskCreated { task_id, description };
    if let Err(e) = broker.publish_event(event, source).await {
        log_warn!("pub_sub", "Failed to publish task created event: {}", e);
    }
}

pub async fn publish_task_completed(task_id: String, success: bool, execution_time_ms: u64, source: String) {
    let broker = get_global_event_broker();
    let event = Event::TaskCompleted { task_id, success, execution_time_ms };
    if let Err(e) = broker.publish_event(event, source).await {
        log_warn!("pub_sub", "Failed to publish task completed event: {}", e);
    }
}

pub async fn publish_safety_violation(tool_name: String, reason: String, source: String) {
    let broker = get_global_event_broker();
    let event = Event::SafetyViolation { tool_name, reason };
    if let Err(e) = broker.publish_event(event, source).await {
        log_warn!("pub_sub", "Failed to publish safety violation event: {}", e);
    }
}