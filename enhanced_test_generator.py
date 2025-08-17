#!/usr/bin/env python3
"""
Enhanced CAI Test Generator with Complex, Real-World Software Development Scenarios

Generates 100 diverse, complex prompts that simulate actual software development tasks
with multi-step requirements, realistic constraints, and production-quality expectations.
"""

import json
import uuid
import random
from typing import List, Dict, Any
from dataclasses import dataclass

@dataclass
class ComplexTestCase:
    test_id: str
    category: str
    difficulty: str
    prompt: str
    expected_outputs: Dict[str, Any]
    setup_requirements: Dict[str, Any]
    timeout_seconds: int
    tags: List[str]
    description: str
    complexity_factors: List[str]

class EnhancedTestGenerator:
    def __init__(self):
        self.languages = ["python", "javascript", "rust", "go", "java", "typescript"]
        self.complexity_levels = ["intermediate", "advanced", "expert"]
        self.categories = [
            "full_stack_development",
            "system_design", 
            "performance_optimization",
            "security_implementation",
            "data_processing",
            "api_development",
            "microservices",
            "database_design",
            "devops_automation",
            "testing_frameworks"
        ]

    def generate_full_stack_prompts(self) -> List[ComplexTestCase]:
        """Generate complex full-stack development scenarios."""
        prompts = [
            {
                "prompt": """Build a complete REST API for a task management system with the following requirements:
1. User authentication with JWT tokens
2. CRUD operations for tasks with categories and priorities
3. Task assignment to multiple users
4. Real-time notifications using WebSockets
5. Database schema with proper relationships
6. Input validation and error handling
7. API documentation with examples
8. Unit and integration tests
Include proper project structure, configuration files, and deployment instructions.""",
                "complexity_factors": ["authentication", "real-time", "database_design", "testing", "documentation"],
                "timeout_seconds": 120
            },
            {
                "prompt": """Create a microservices-based e-commerce platform with:
1. User service (registration, authentication, profiles)
2. Product catalog service with search and filtering
3. Shopping cart service with session management
4. Order processing service with payment integration
5. Inventory management with stock tracking
6. API Gateway for routing and rate limiting
7. Service discovery and load balancing
8. Distributed logging and monitoring
9. Docker containers and orchestration
10. CI/CD pipeline configuration
Include inter-service communication, data consistency, and fault tolerance.""",
                "complexity_factors": ["microservices", "distributed_systems", "payment_processing", "containerization", "monitoring"],
                "timeout_seconds": 180
            },
            {
                "prompt": """Develop a real-time collaborative code editor similar to VS Code Live Share:
1. WebSocket-based real-time synchronization
2. Operational transformation for conflict resolution
3. User presence and cursor tracking
4. Syntax highlighting for multiple languages
5. File explorer with project structure
6. Live chat and voice call integration
7. Code execution environment (sandbox)
8. Version control integration
9. Plugin architecture
10. Performance optimization for large files
Include proper state management, security considerations, and scalability design.""",
                "complexity_factors": ["real-time_collaboration", "operational_transformation", "security", "performance", "plugin_architecture"],
                "timeout_seconds": 150
            }
        ]
        
        return self._create_test_cases("full_stack_development", prompts, ["advanced", "expert"])

    def generate_system_design_prompts(self) -> List[ComplexTestCase]:
        """Generate complex system design and architecture scenarios."""
        prompts = [
            {
                "prompt": """Design and implement a distributed caching system similar to Redis Cluster:
1. Consistent hashing for data distribution
2. Replication for high availability
3. Automatic failover and recovery
4. Memory management with LRU/LFU eviction
5. Pub/Sub messaging system
6. Transaction support with ACID properties
7. Cluster monitoring and health checks
8. Data persistence with snapshots
9. Client libraries for multiple languages
10. Performance benchmarking suite
Include protocol design, network communication, and fault tolerance mechanisms.""",
                "complexity_factors": ["distributed_systems", "consistent_hashing", "replication", "memory_management", "networking"],
                "timeout_seconds": 180
            },
            {
                "prompt": """Build a scalable message queue system like Apache Kafka:
1. Topic partitioning and replication
2. Producer and consumer APIs
3. Offset management and tracking
4. Dead letter queues for failed messages
5. Schema registry for message validation
6. Stream processing capabilities
7. Cluster coordination with consensus
8. Performance monitoring and metrics
9. Security with authentication/authorization
10. Multi-datacenter replication
Include durability guarantees, ordering semantics, and backpressure handling.""",
                "complexity_factors": ["message_queuing", "stream_processing", "consensus_algorithms", "multi_datacenter", "performance_tuning"],
                "timeout_seconds": 180
            },
            {
                "prompt": """Create a container orchestration platform similar to Kubernetes:
1. Pod scheduling and resource allocation
2. Service discovery and load balancing
3. Auto-scaling based on metrics
4. Rolling deployments and rollbacks
5. Persistent volume management
6. Network policies and security
7. Configuration and secret management
8. Monitoring and logging aggregation
9. Multi-cluster federation
10. Operator framework for custom resources
Include controller patterns, API design, and cluster state management.""",
                "complexity_factors": ["container_orchestration", "resource_scheduling", "auto_scaling", "network_policies", "operator_patterns"],
                "timeout_seconds": 200
            }
        ]
        
        return self._create_test_cases("system_design", prompts, ["expert"])

    def generate_performance_optimization_prompts(self) -> List[ComplexTestCase]:
        """Generate performance optimization and profiling scenarios."""
        prompts = [
            {
                "prompt": """Optimize a high-frequency trading system for ultra-low latency:
1. Memory pool allocation to avoid GC pauses
2. Lock-free data structures for concurrent access
3. CPU cache optimization and data locality
4. Network optimization with kernel bypass
5. Real-time profiling and metrics collection
6. JIT compilation optimization techniques
7. NUMA-aware memory allocation
8. Hardware-specific optimizations (SIMD, etc.)
9. Latency measurement and statistical analysis
10. Stress testing under various market conditions
Include benchmarking results, profiling reports, and optimization trade-offs.""",
                "complexity_factors": ["low_latency", "lock_free_programming", "memory_optimization", "hardware_optimization", "profiling"],
                "timeout_seconds": 150
            },
            {
                "prompt": """Build a high-performance analytics engine for processing terabytes of data:
1. Columnar storage format with compression
2. Vectorized query execution engine
3. Parallel processing with work-stealing
4. Memory-mapped file I/O optimization
5. Query optimization and cost-based planning
6. Adaptive indexing strategies
7. Approximate query processing
8. Resource management and memory budgeting
9. Distributed execution coordination
10. Real-time stream processing integration
Include performance benchmarks, memory usage analysis, and scalability tests.""",
                "complexity_factors": ["big_data_processing", "vectorized_execution", "query_optimization", "memory_mapping", "distributed_computing"],
                "timeout_seconds": 180
            }
        ]
        
        return self._create_test_cases("performance_optimization", prompts, ["advanced", "expert"])

    def generate_security_implementation_prompts(self) -> List[ComplexTestCase]:
        """Generate security-focused development scenarios."""
        prompts = [
            {
                "prompt": """Implement a zero-trust security framework for microservices:
1. mTLS certificate management and rotation
2. Service mesh with security policies
3. JWT token validation and refresh
4. Role-based access control (RBAC)
5. API rate limiting and DDoS protection
6. Secrets management with encryption
7. Security scanning and vulnerability assessment
8. Audit logging and compliance reporting
9. Network segmentation and firewalls
10. Incident response automation
Include threat modeling, penetration testing, and security best practices.""",
                "complexity_factors": ["zero_trust", "mtls", "rbac", "secrets_management", "threat_modeling"],
                "timeout_seconds": 150
            },
            {
                "prompt": """Create a secure blockchain-based voting system:
1. Cryptographic vote encryption and verification
2. Zero-knowledge proofs for privacy
3. Smart contracts for vote tallying
4. Distributed consensus mechanism
5. Voter identity verification
6. Ballot auditing and transparency
7. Protection against various attack vectors
8. Scalability for millions of voters
9. Accessibility compliance
10. Legal and regulatory compliance
Include cryptographic protocols, security analysis, and formal verification.""",
                "complexity_factors": ["blockchain", "cryptography", "zero_knowledge_proofs", "consensus_mechanisms", "formal_verification"],
                "timeout_seconds": 200
            }
        ]
        
        return self._create_test_cases("security_implementation", prompts, ["expert"])

    def generate_data_processing_prompts(self) -> List[ComplexTestCase]:
        """Generate complex data processing and ML scenarios."""
        prompts = [
            {
                "prompt": """Build a real-time fraud detection system for financial transactions:
1. Stream processing pipeline for live transactions
2. Machine learning models for anomaly detection
3. Feature engineering with sliding windows
4. Model training and online learning
5. A/B testing framework for model comparison
6. Feedback loop for false positive reduction
7. Explainable AI for regulatory compliance
8. Data privacy and encryption
9. High availability and disaster recovery
10. Performance monitoring and alerting
Include data pipeline architecture, model evaluation metrics, and regulatory considerations.""",
                "complexity_factors": ["stream_processing", "machine_learning", "anomaly_detection", "explainable_ai", "regulatory_compliance"],
                "timeout_seconds": 180
            },
            {
                "prompt": """Create a distributed data lake platform for multi-petabyte analytics:
1. Data ingestion from multiple sources (batch/stream)
2. Schema evolution and data governance
3. Partitioning strategies for performance
4. Data quality monitoring and validation
5. ETL/ELT pipeline orchestration
6. Multi-tenant access control
7. Cost optimization and lifecycle management
8. Metadata catalog and lineage tracking
9. SQL and NoSQL query engines
10. Integration with ML/AI platforms
Include data architecture design, cost analysis, and governance policies.""",
                "complexity_factors": ["data_lake", "schema_evolution", "data_governance", "pipeline_orchestration", "multi_tenant"],
                "timeout_seconds": 180
            }
        ]
        
        return self._create_test_cases("data_processing", prompts, ["advanced", "expert"])

    def generate_api_development_prompts(self) -> List[ComplexTestCase]:
        """Generate advanced API development scenarios."""
        prompts = [
            {
                "prompt": """Design and implement a GraphQL API for a social media platform:
1. Schema design with unions and interfaces
2. Efficient data fetching with DataLoader
3. Real-time subscriptions for live updates
4. Authentication and authorization middleware
5. Rate limiting and query complexity analysis
6. Caching strategies (Redis, CDN)
7. Error handling and custom scalars
8. API versioning and deprecation
9. Performance monitoring and tracing
10. Federation for microservices
Include schema documentation, performance benchmarks, and security considerations.""",
                "complexity_factors": ["graphql", "real_time_subscriptions", "dataloader_pattern", "query_complexity", "federation"],
                "timeout_seconds": 150
            },
            {
                "prompt": """Build a comprehensive API gateway with advanced features:
1. Dynamic routing and load balancing
2. Protocol translation (REST, GraphQL, gRPC)
3. Request/response transformation
4. Circuit breaker and retry policies
5. Authentication aggregation
6. API versioning and blue-green deployments
7. Analytics and usage metrics
8. Developer portal with documentation
9. Webhook management and delivery
10. Multi-cloud deployment support
Include configuration management, monitoring dashboards, and developer experience tools.""",
                "complexity_factors": ["api_gateway", "protocol_translation", "circuit_breaker", "blue_green_deployment", "developer_portal"],
                "timeout_seconds": 180
            }
        ]
        
        return self._create_test_cases("api_development", prompts, ["advanced", "expert"])

    def generate_additional_complex_prompts(self) -> List[ComplexTestCase]:
        """Generate remaining complex scenarios across various domains."""
        additional_prompts = []
        
        # Database Design & Optimization
        additional_prompts.extend([
            {
                "category": "database_design",
                "prompt": """Design a time-series database optimized for IoT sensor data:
1. Custom storage engine with columnar compression
2. Time-based partitioning and retention policies
3. Aggregation functions and downsampling
4. High-throughput ingestion (millions of points/sec)
5. Query optimization for time ranges
6. Replication and clustering
7. Schema flexibility for different sensor types
8. Real-time alerting on anomalies
9. Integration with visualization tools
10. Backup and disaster recovery
Include performance benchmarks, storage efficiency analysis, and scalability testing.""",
                "complexity_factors": ["time_series", "columnar_storage", "high_throughput", "query_optimization", "clustering"],
                "timeout_seconds": 180,
                "difficulty": "expert"
            }
        ])
        
        # DevOps Automation
        additional_prompts.extend([
            {
                "category": "devops_automation",
                "prompt": """Create an intelligent CI/CD platform with ML-powered optimizations:
1. Pipeline optimization using historical data
2. Predictive failure detection
3. Dynamic resource allocation
4. Automatic dependency resolution
5. Security scanning integration
6. Multi-cloud deployment strategies
7. Rollback automation with health checks
8. Cost optimization recommendations
9. Developer productivity analytics
10. Infrastructure as code management
Include ML models for optimization, monitoring dashboards, and cost analysis.""",
                "complexity_factors": ["ml_optimization", "predictive_analytics", "multi_cloud", "infrastructure_as_code", "cost_optimization"],
                "timeout_seconds": 180,
                "difficulty": "expert"
            }
        ])
        
        # Testing Frameworks
        additional_prompts.extend([
            {
                "category": "testing_frameworks",
                "prompt": """Build an AI-powered testing framework for web applications:
1. Visual regression testing with ML
2. Automatic test case generation
3. Self-healing test scripts
4. Cross-browser compatibility testing
5. Performance testing with load generation
6. Accessibility compliance validation
7. API contract testing
8. Test data management and privacy
9. Parallel execution optimization
10. Intelligent test result analysis
Include ML models for test generation, reporting dashboards, and integration examples.""",
                "complexity_factors": ["ai_testing", "visual_regression", "self_healing", "cross_browser", "test_generation"],
                "timeout_seconds": 150,
                "difficulty": "advanced"
            }
        ])
        
        return [self._create_single_test_case(prompt_data["category"], prompt_data) for prompt_data in additional_prompts]

    def _create_test_cases(self, category: str, prompts: List[Dict], difficulties: List[str]) -> List[ComplexTestCase]:
        """Create test cases for a category."""
        test_cases = []
        for i, prompt_data in enumerate(prompts):
            difficulty = difficulties[i % len(difficulties)]
            test_cases.append(self._create_single_test_case(category, {**prompt_data, "difficulty": difficulty}))
        return test_cases

    def _create_single_test_case(self, category: str, prompt_data: Dict) -> ComplexTestCase:
        """Create a single test case."""
        test_id = str(uuid.uuid4())
        language = random.choice(self.languages)
        
        return ComplexTestCase(
            test_id=test_id,
            category=category,
            difficulty=prompt_data.get("difficulty", "advanced"),
            prompt=prompt_data["prompt"],
            expected_outputs={
                "files_created": [f"main.{self._get_extension(language)}", "README.md", "tests/"],
                "project_structure": True,
                "documentation": True,
                "tests": True,
                "success_criteria": "functional_with_quality",
                "evaluation_method": "comprehensive_review",
                "quality_requirements": [
                    "proper_error_handling",
                    "performance_considerations", 
                    "security_best_practices",
                    "comprehensive_documentation",
                    "production_ready_code"
                ]
            },
            setup_requirements={
                "language": language,
                "complexity_level": prompt_data.get("difficulty", "advanced"),
                "requires_multiple_files": True,
                "requires_configuration": True
            },
            timeout_seconds=prompt_data.get("timeout_seconds", 150),
            tags=[language, category, prompt_data.get("difficulty", "advanced")] + prompt_data.get("complexity_factors", []),
            description=f"Complex {category} scenario requiring {prompt_data.get('difficulty', 'advanced')} implementation",
            complexity_factors=prompt_data.get("complexity_factors", [])
        )

    def _get_extension(self, language: str) -> str:
        """Get file extension for language."""
        extensions = {
            "python": "py",
            "javascript": "js", 
            "typescript": "ts",
            "rust": "rs",
            "go": "go",
            "java": "java"
        }
        return extensions.get(language, "py")

    def generate_complete_test_suite(self) -> Dict[str, Any]:
        """Generate the complete enhanced test suite."""
        all_test_cases = []
        
        # Generate complex prompts by category
        all_test_cases.extend(self.generate_full_stack_prompts())
        all_test_cases.extend(self.generate_system_design_prompts()) 
        all_test_cases.extend(self.generate_performance_optimization_prompts())
        all_test_cases.extend(self.generate_security_implementation_prompts())
        all_test_cases.extend(self.generate_data_processing_prompts())
        all_test_cases.extend(self.generate_api_development_prompts())
        all_test_cases.extend(self.generate_additional_complex_prompts())
        
        # Ensure we have exactly 100 tests
        while len(all_test_cases) < 100:
            # Generate additional intermediate complexity tests
            additional = self._generate_intermediate_prompts()
            all_test_cases.extend(additional[:100 - len(all_test_cases)])
        
        # Trim to exactly 100 tests
        all_test_cases = all_test_cases[:100]
        
        # Convert to dictionary format
        test_dict_list = []
        for test_case in all_test_cases:
            test_dict_list.append({
                "test_id": test_case.test_id,
                "category": test_case.category,
                "difficulty": test_case.difficulty,
                "prompt": test_case.prompt,
                "expected_outputs": test_case.expected_outputs,
                "setup_requirements": test_case.setup_requirements,
                "timeout_seconds": test_case.timeout_seconds,
                "tags": test_case.tags,
                "description": test_case.description,
                "complexity_factors": test_case.complexity_factors
            })
        
        # Generate metadata
        category_counts = {}
        difficulty_counts = {}
        language_counts = {}
        
        for test in test_dict_list:
            # Category counts
            cat = test["category"]
            category_counts[cat] = category_counts.get(cat, 0) + 1
            
            # Difficulty counts
            diff = test["difficulty"] 
            difficulty_counts[diff] = difficulty_counts.get(diff, 0) + 1
            
            # Language counts
            lang = test["setup_requirements"].get("language", "unknown")
            language_counts[lang] = language_counts.get(lang, 0) + 1
        
        return {
            "metadata": {
                "total_tests": len(test_dict_list),
                "categories": category_counts,
                "difficulty_distribution": difficulty_counts,
                "languages_covered": list(language_counts.keys()),
                "language_distribution": language_counts,
                "complexity_features": [
                    "multi_step_requirements",
                    "production_quality_expectations", 
                    "real_world_scenarios",
                    "performance_considerations",
                    "security_requirements",
                    "scalability_challenges",
                    "integration_complexity"
                ],
                "average_timeout": sum(t["timeout_seconds"] for t in test_dict_list) / len(test_dict_list),
                "generation_timestamp": "2025-08-15T13:30:00Z",
                "generator_version": "2.0_enhanced"
            },
            "tests": test_dict_list
        }

    def _generate_intermediate_prompts(self) -> List[ComplexTestCase]:
        """Generate additional intermediate complexity prompts to reach 100 total."""
        intermediate_prompts = [
            {
                "category": "microservices",
                "prompt": """Build a distributed logging system for microservices:
1. Log aggregation from multiple services
2. Structured logging with correlation IDs
3. Real-time log streaming and search
4. Log retention and archival policies
5. Alerting on error patterns
6. Performance impact monitoring
7. Multi-tenant log isolation
8. Integration with observability tools
Include log parsing, indexing strategies, and visualization components.""",
                "complexity_factors": ["distributed_logging", "log_aggregation", "real_time_search", "multi_tenant"],
                "timeout_seconds": 120,
                "difficulty": "intermediate"
            },
            {
                "category": "microservices", 
                "prompt": """Create a service mesh control plane:
1. Service discovery and registration
2. Traffic routing and load balancing
3. Circuit breaker implementation
4. Metrics collection and monitoring
5. Security policy enforcement
6. Configuration management
7. Health checking and failover
8. Multi-cluster support
Include proxy configuration, control plane APIs, and observability features.""",
                "complexity_factors": ["service_mesh", "traffic_routing", "circuit_breaker", "security_policies"],
                "timeout_seconds": 150,
                "difficulty": "advanced"
            }
        ]
        
        return [self._create_single_test_case(prompt_data["category"], prompt_data) for prompt_data in intermediate_prompts]

def main():
    """Generate enhanced test suite and save to file."""
    generator = EnhancedTestGenerator()
    test_suite = generator.generate_complete_test_suite()
    
    # Save to file
    with open('enhanced_cai_test_suite.json', 'w') as f:
        json.dump(test_suite, f, indent=2)
    
    print(f"Generated enhanced test suite with {test_suite['metadata']['total_tests']} complex tests")
    print(f"Categories: {list(test_suite['metadata']['categories'].keys())}")
    print(f"Difficulty distribution: {test_suite['metadata']['difficulty_distribution']}")
    print(f"Languages: {test_suite['metadata']['languages_covered']}")
    print(f"Average timeout: {test_suite['metadata']['average_timeout']:.1f} seconds")
    print("Enhanced test suite saved to 'enhanced_cai_test_suite.json'")

if __name__ == "__main__":
    main()