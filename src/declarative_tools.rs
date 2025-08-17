use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::logger::{log_debug, log_info};
use crate::multi_agent::AgentCapability;

/// OpenAPI 3.0 style tool schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: ToolCategory,
    pub capabilities: Vec<AgentCapability>,
    pub parameters: ParameterSchema,
    pub returns: ReturnSchema,
    pub examples: Vec<ToolExample>,
    pub safety_level: ToolSafetyLevel,
    pub implementation: ToolImplementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    FileSystem,
    Network,
    CodeProcessing,
    DataTransformation,
    SystemInteraction,
    Planning,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub properties: HashMap<String, PropertyDefinition>,
    pub required: Vec<String>,
    pub additional_properties: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDefinition {
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    pub description: String,
    pub format: Option<String>,
    pub enum_values: Option<Vec<Value>>,
    pub default: Option<Value>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnSchema {
    #[serde(rename = "type")]
    pub return_type: PropertyType,
    pub description: String,
    pub properties: Option<HashMap<String, PropertyDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: Value,
    pub output: Value,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSafetyLevel {
    Safe,
    RequiresApproval,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolImplementation {
    LocalFunction(String),
    McpServer { server: String, tool: String },
    HttpEndpoint { url: String, method: String },
    Custom(Value),
}

/// Tool execution context for declarative tools
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub user_id: Option<String>,
    pub working_directory: String,
    pub environment: HashMap<String, String>,
    pub metadata: HashMap<String, Value>,
}

/// Result of declarative tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub result: Value,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
}

/// Declarative tool registry for managing tools defined via schemas
pub struct DeclarativeToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolSchema>>>,
    executors: Arc<RwLock<HashMap<String, Box<dyn ToolExecutor>>>>,
}

/// Trait for executing declarative tools
pub trait ToolExecutor: Send + Sync {
    fn execute(
        &self,
        tool: &ToolSchema,
        parameters: Value,
        context: &ToolContext,
    ) -> Result<ToolExecutionResult>;
}

impl DeclarativeToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a tool schema
    pub fn register_tool(&self, schema: ToolSchema) -> Result<()> {
        let mut tools = self.tools.write().map_err(|_| anyhow!("Failed to acquire write lock"))?;
        
        log_info!("declarative_tools", "Registering tool: {} v{}", schema.name, schema.version);
        
        // Validate schema
        self.validate_schema(&schema)?;
        
        tools.insert(schema.name.clone(), schema);
        Ok(())
    }
    
    /// Register a tool executor
    pub fn register_executor(&self, category: &str, executor: Box<dyn ToolExecutor>) -> Result<()> {
        let mut executors = self.executors.write().map_err(|_| anyhow!("Failed to acquire write lock"))?;
        executors.insert(category.to_string(), executor);
        log_info!("declarative_tools", "Registered executor for category: {}", category);
        Ok(())
    }
    
    /// Get all registered tools
    pub fn list_tools(&self) -> Result<Vec<ToolSchema>> {
        let tools = self.tools.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
        Ok(tools.values().cloned().collect())
    }
    
    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Result<Option<ToolSchema>> {
        let tools = self.tools.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
        Ok(tools.get(name).cloned())
    }
    
    /// Find tools by capability
    pub fn find_tools_by_capability(&self, capability: &AgentCapability) -> Result<Vec<ToolSchema>> {
        let tools = self.tools.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
        let matching_tools = tools
            .values()
            .filter(|tool| tool.capabilities.contains(capability))
            .cloned()
            .collect();
        Ok(matching_tools)
    }
    
    /// Execute a tool by name
    pub fn execute_tool(
        &self,
        name: &str,
        parameters: Value,
        context: &ToolContext,
    ) -> Result<ToolExecutionResult> {
        let start_time = std::time::Instant::now();
        
        // Get tool schema
        let tool = {
            let tools = self.tools.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
            tools.get(name).cloned().ok_or_else(|| anyhow!("Tool '{}' not found", name))?
        };
        
        // Validate parameters
        self.validate_parameters(&tool, &parameters)?;
        
        // Find appropriate executor
        let category_key = format!("{:?}", tool.category);
        let executors = self.executors.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
        let executor = executors.get(&category_key)
            .ok_or_else(|| anyhow!("No executor found for tool category: {:?}", tool.category))?
            .as_ref();
        
        log_debug!("declarative_tools", "Executing tool '{}' with executor for category: {:?}", name, tool.category);
        
        // Execute tool
        let mut result = executor.execute(&tool, parameters, context)?;
        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        log_info!("declarative_tools", "Tool '{}' executed successfully in {}ms", name, result.execution_time_ms);
        
        Ok(result)
    }
    
    /// Generate OpenAPI 3.0 specification for all registered tools
    pub fn generate_openapi_spec(&self) -> Result<Value> {
        let tools = self.tools.read().map_err(|_| anyhow!("Failed to acquire read lock"))?;
        
        let mut paths = json!({});
        let components = json!({
            "schemas": {}
        });
        
        for tool in tools.values() {
            // Create path for each tool
            let path_key = format!("/tools/{}", tool.name);
            paths[&path_key] = json!({
                "post": {
                    "summary": tool.description,
                    "operationId": tool.name,
                    "tags": [format!("{:?}", tool.category)],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": self.parameter_schema_to_json_schema(&tool.parameters)
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": tool.returns.description,
                            "content": {
                                "application/json": {
                                    "schema": self.return_schema_to_json_schema(&tool.returns)
                                }
                            }
                        }
                    }
                }
            });
            
            // Add examples if available
            if !tool.examples.is_empty() {
                paths[&path_key]["post"]["examples"] = json!(tool.examples);
            }
        }
        
        Ok(json!({
            "openapi": "3.0.0",
            "info": {
                "title": "CAI Declarative Tools API",
                "version": "1.0.0",
                "description": "Auto-generated OpenAPI specification for CAI declarative tools"
            },
            "paths": paths,
            "components": components
        }))
    }
    
    /// Validate tool schema
    fn validate_schema(&self, schema: &ToolSchema) -> Result<()> {
        if schema.name.is_empty() {
            return Err(anyhow!("Tool name cannot be empty"));
        }
        
        if schema.description.is_empty() {
            return Err(anyhow!("Tool description cannot be empty"));
        }
        
        if schema.parameters.required.iter().any(|req| !schema.parameters.properties.contains_key(req)) {
            return Err(anyhow!("Required parameter not defined in properties"));
        }
        
        Ok(())
    }
    
    /// Validate parameters against schema
    fn validate_parameters(&self, tool: &ToolSchema, parameters: &Value) -> Result<()> {
        let params_obj = parameters.as_object()
            .ok_or_else(|| anyhow!("Parameters must be a JSON object"))?;
        
        // Check required parameters
        for required in &tool.parameters.required {
            if !params_obj.contains_key(required) {
                return Err(anyhow!("Missing required parameter: {}", required));
            }
        }
        
        // Validate parameter types and constraints
        for (param_name, param_value) in params_obj {
            if let Some(prop_def) = tool.parameters.properties.get(param_name) {
                self.validate_property_value(param_name, param_value, prop_def)?;
            } else if !tool.parameters.additional_properties {
                return Err(anyhow!("Additional parameter '{}' not allowed", param_name));
            }
        }
        
        Ok(())
    }
    
    /// Validate individual property value
    fn validate_property_value(&self, name: &str, value: &Value, definition: &PropertyDefinition) -> Result<()> {
        match (&definition.property_type, value) {
            (PropertyType::String, Value::String(s)) => {
                if let Some(pattern) = &definition.pattern {
                    let regex = regex::Regex::new(pattern)?;
                    if !regex.is_match(s) {
                        return Err(anyhow!("Parameter '{}' does not match pattern: {}", name, pattern));
                    }
                }
            }
            (PropertyType::Number, Value::Number(_)) => {}
            (PropertyType::Integer, Value::Number(n)) => {
                if !n.is_i64() {
                    return Err(anyhow!("Parameter '{}' must be an integer", name));
                }
            }
            (PropertyType::Boolean, Value::Bool(_)) => {}
            (PropertyType::Array, Value::Array(_)) => {}
            (PropertyType::Object, Value::Object(_)) => {}
            _ => {
                return Err(anyhow!("Parameter '{}' has invalid type", name));
            }
        }
        
        Ok(())
    }
    
    /// Convert parameter schema to JSON Schema
    fn parameter_schema_to_json_schema(&self, schema: &ParameterSchema) -> Value {
        let mut json_schema = json!({
            "type": "object",
            "properties": {},
            "required": schema.required,
            "additionalProperties": schema.additional_properties
        });
        
        for (prop_name, prop_def) in &schema.properties {
            json_schema["properties"][prop_name] = self.property_definition_to_json_schema(prop_def);
        }
        
        json_schema
    }
    
    /// Convert return schema to JSON Schema
    fn return_schema_to_json_schema(&self, schema: &ReturnSchema) -> Value {
        let mut json_schema = json!({
            "type": format!("{:?}", schema.return_type).to_lowercase(),
            "description": schema.description
        });
        
        if let Some(properties) = &schema.properties {
            json_schema["properties"] = json!({});
            for (prop_name, prop_def) in properties {
                json_schema["properties"][prop_name] = self.property_definition_to_json_schema(prop_def);
            }
        }
        
        json_schema
    }
    
    /// Convert property definition to JSON Schema
    fn property_definition_to_json_schema(&self, definition: &PropertyDefinition) -> Value {
        let mut schema = json!({
            "type": format!("{:?}", definition.property_type).to_lowercase(),
            "description": definition.description
        });
        
        if let Some(format) = &definition.format {
            schema["format"] = json!(format);
        }
        
        if let Some(enum_values) = &definition.enum_values {
            schema["enum"] = json!(enum_values);
        }
        
        if let Some(default) = &definition.default {
            schema["default"] = default.clone();
        }
        
        if let Some(min) = definition.minimum {
            schema["minimum"] = json!(min);
        }
        
        if let Some(max) = definition.maximum {
            schema["maximum"] = json!(max);
        }
        
        if let Some(pattern) = &definition.pattern {
            schema["pattern"] = json!(pattern);
        }
        
        schema
    }
}

/// Built-in executor for local function tools
pub struct LocalFunctionExecutor {
    functions: HashMap<String, Box<dyn Fn(Value) -> Result<Value> + Send + Sync>>,
}

impl LocalFunctionExecutor {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
    
    pub fn register_function<F>(&mut self, name: String, function: F)
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        self.functions.insert(name, Box::new(function));
    }
}

impl ToolExecutor for LocalFunctionExecutor {
    fn execute(
        &self,
        tool: &ToolSchema,
        parameters: Value,
        _context: &ToolContext,
    ) -> Result<ToolExecutionResult> {
        if let ToolImplementation::LocalFunction(function_name) = &tool.implementation {
            if let Some(function) = self.functions.get(function_name) {
                match function(parameters) {
                    Ok(result) => Ok(ToolExecutionResult {
                        success: true,
                        result,
                        error_message: None,
                        execution_time_ms: 0, // Will be set by registry
                        metadata: HashMap::new(),
                    }),
                    Err(e) => Ok(ToolExecutionResult {
                        success: false,
                        result: Value::Null,
                        error_message: Some(e.to_string()),
                        execution_time_ms: 0,
                        metadata: HashMap::new(),
                    }),
                }
            } else {
                Err(anyhow!("Function '{}' not found in local executor", function_name))
            }
        } else {
            Err(anyhow!("Tool implementation is not a local function"))
        }
    }
}

/// Helper functions for creating common tool schemas
pub mod builders {
    use super::*;
    
    pub fn create_file_operation_tool(
        name: &str,
        description: &str,
        function_name: &str,
        required_params: Vec<&str>,
    ) -> ToolSchema {
        let mut properties = HashMap::new();
        
        // Common file operation parameters
        properties.insert("path".to_string(), PropertyDefinition {
            property_type: PropertyType::String,
            description: "File or directory path".to_string(),
            format: Some("path".to_string()),
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            pattern: None,
        });
        
        if required_params.contains(&"content") {
            properties.insert("content".to_string(), PropertyDefinition {
                property_type: PropertyType::String,
                description: "File content".to_string(),
                format: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                pattern: None,
            });
        }
        
        ToolSchema {
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            category: ToolCategory::FileSystem,
            capabilities: vec![AgentCapability::FileSystem],
            parameters: ParameterSchema {
                properties,
                required: required_params.iter().map(|s| s.to_string()).collect(),
                additional_properties: true,
            },
            returns: ReturnSchema {
                return_type: PropertyType::Object,
                description: "Operation result".to_string(),
                properties: None,
            },
            examples: vec![],
            safety_level: ToolSafetyLevel::RequiresApproval,
            implementation: ToolImplementation::LocalFunction(function_name.to_string()),
        }
    }
    
    pub fn create_web_operation_tool(
        name: &str,
        description: &str,
        function_name: &str,
    ) -> ToolSchema {
        let mut properties = HashMap::new();
        
        properties.insert("url".to_string(), PropertyDefinition {
            property_type: PropertyType::String,
            description: "Target URL".to_string(),
            format: Some("uri".to_string()),
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            pattern: Some(r"^https?://.*".to_string()),
        });
        
        ToolSchema {
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            category: ToolCategory::Network,
            capabilities: vec![AgentCapability::WebOperations],
            parameters: ParameterSchema {
                properties,
                required: vec!["url".to_string()],
                additional_properties: true,
            },
            returns: ReturnSchema {
                return_type: PropertyType::Object,
                description: "Web operation result".to_string(),
                properties: None,
            },
            examples: vec![],
            safety_level: ToolSafetyLevel::Safe,
            implementation: ToolImplementation::LocalFunction(function_name.to_string()),
        }
    }
}