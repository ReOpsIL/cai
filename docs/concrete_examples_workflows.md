# Concrete Examples and Workflows

## 1. Complete Task Execution Example

### Scenario: "Add user authentication to a Node.js Express app"

#### 1.1 Initial Task Analysis (Level 1 - Task Reasoning)
```rust
// Input processing
let user_input = "Add user authentication to my Express app with login, signup, and protected routes";

// Task reasoning output
TaskAnalysis {
    primary_goal: "Implement comprehensive user authentication system",
    requirements: vec![
        "User registration functionality",
        "User login/logout functionality", 
        "Protected route middleware",
        "Session management",
        "Password security (hashing)",
    ],
    constraints: vec![
        "Must work with existing Express.js setup",
        "Should follow security best practices",
        "Maintain existing API structure",
    ],
    dependencies: vec![
        "Express.js framework",
        "Database for user storage",
        "Password hashing library",
        "Session management middleware",
    ],
    estimated_complexity: ComplexityScore::Medium(7.5),
    confidence: 0.85,
}
```

#### 1.2 Strategy Planning (Level 2 - Strategy Reasoning)
```rust
// Generated approaches
let approaches = vec![
    Approach {
        id: "passport-local",
        strategy: "Use Passport.js with local strategy",
        steps: vec![
            "Install passport and bcrypt dependencies",
            "Configure passport local strategy",
            "Create user model and database schema",
            "Implement registration endpoint",
            "Implement login endpoint", 
            "Add authentication middleware",
            "Protect existing routes",
        ],
        risk_factors: vec![
            RiskFactor::new("Passport configuration complexity", 0.3),
            RiskFactor::new("Database schema conflicts", 0.2),
        ],
        estimated_time: Duration::from_mins(45),
        confidence: 0.9,
    },
    Approach {
        id: "jwt-custom",
        strategy: "Custom JWT implementation",
        steps: vec![
            "Install jsonwebtoken and bcrypt",
            "Create JWT utilities",
            "Implement custom auth middleware",
            "Create registration endpoint",
            "Create login endpoint with JWT",
            "Protect routes with JWT verification",
        ],
        risk_factors: vec![
            RiskFactor::new("Custom security implementation", 0.7),
            RiskFactor::new("JWT best practices compliance", 0.4),
        ],
        estimated_time: Duration::from_mins(60),
        confidence: 0.7,
    },
];

// Strategy selection
let selected = ApproachSelector::select_optimal(approaches);
// Result: passport-local (higher confidence, lower risk)
```

#### 1.3 Meta-Reasoning Assessment (Level 3)
```rust
MetaAnalysis {
    confidence_breakdown: ConfidenceBreakdown {
        task_understanding: 0.95,
        strategy_selection: 0.90,
        execution_planning: 0.85,
        risk_assessment: 0.80,
        outcome_prediction: 0.75,
    },
    adaptation_triggers: vec![
        AdaptationTrigger {
            condition: "If Passport.js installation fails",
            alternative: "Switch to JWT approach",
            threshold: 0.5,
        },
        AdaptationTrigger {
            condition: "If database conflicts arise", 
            alternative: "Create new user table schema",
            threshold: 0.6,
        },
    ],
    learning_opportunities: vec![
        "Express.js authentication patterns",
        "Password security best practices",
        "Session management approaches",
    ],
}
```

### 1.4 Execution with Hypothesis-Driven Development

#### Phase 1: Environment Analysis
```rust
// Hypothesis: "This is a standard Express.js app with existing routes"
let hypothesis_1 = Hypothesis {
    statement: "Application uses Express.js with file-based routing structure",
    evidence_criteria: vec![
        "package.json contains express dependency",
        "app.js or server.js file exists", 
        "routes/ directory exists",
    ],
    confidence: 0.8,
};

// Tool execution: file_operations
let package_json = file_tool.read("package.json").await?;
let app_structure = file_tool.list_directory(".").await?;

// Evidence collection
if package_json.contains("express") && app_structure.contains("app.js") {
    hypothesis_1.status = HypothesisStatus::Confirmed;
    confidence_tracker.update(0.95); // Increased confidence
}
```

#### Phase 2: Dependency Installation
```rust
// Self-questioning before action
let pre_action_reflection = vec![
    "What dependencies do I need to install?",
    "Are there any version conflicts to consider?",
    "Should I check the existing package.json for conflicting dependencies?",
];

// Hypothesis: "Standard npm install will work without conflicts"
let dependency_hypothesis = Hypothesis {
    statement: "Installing passport, passport-local, bcrypt, express-session will succeed",
    test_criteria: vec![
        "npm install exits with code 0",
        "package.json updated with new dependencies",
        "node_modules contains installed packages",
    ],
};

// Tool execution with error recovery
let install_result = match bash_tool.execute("npm install passport passport-local bcrypt express-session").await {
    Ok(output) => {
        dependency_hypothesis.status = HypothesisStatus::Confirmed;
        output
    },
    Err(e) => {
        // Error recovery: analyze the error and adapt
        let error_analysis = error_analyzer.analyze(&e);
        if error_analysis.category == ErrorCategory::VersionConflict {
            // Adaptive strategy: install specific versions
            bash_tool.execute("npm install passport@^0.6.0 passport-local@^1.0.0 bcrypt@^5.0.0 express-session@^1.17.0").await?
        } else {
            return Err(e);
        }
    }
};
```

#### Phase 3: Implementation with Progressive Validation
```rust
// Step 1: Create user model
let user_model_code = r#"
const mongoose = require('mongoose');
const bcrypt = require('bcrypt');

const userSchema = new mongoose.Schema({
  username: { type: String, required: true, unique: true },
  email: { type: String, required: true, unique: true },
  password: { type: String, required: true },
  createdAt: { type: Date, default: Date.now }
});

userSchema.pre('save', async function(next) {
  if (!this.isModified('password')) return next();
  this.password = await bcrypt.hash(this.password, 12);
  next();
});

userSchema.methods.comparePassword = async function(candidatePassword) {
  return bcrypt.compare(candidatePassword, this.password);
};

module.exports = mongoose.model('User', userSchema);
"#;

file_tool.write("models/User.js", user_model_code).await?;

// Validation checkpoint
let syntax_check = bash_tool.execute("node -c models/User.js").await?;
if !syntax_check.success {
    return Err(ToolError::SyntaxError("User model has syntax errors".to_string()));
}
```

#### Phase 4: Passport Configuration
```rust
// Self-questioning during implementation
let implementation_questions = vec![
    "Does the passport configuration align with the existing app structure?",
    "Are the session settings secure and appropriate?",
    "Will this integrate well with existing middleware?",
];

let passport_config = r#"
const passport = require('passport');
const LocalStrategy = require('passport-local').Strategy;
const User = require('./models/User');

passport.use(new LocalStrategy(
  { usernameField: 'email' },
  async (email, password, done) => {
    try {
      const user = await User.findOne({ email });
      if (!user) {
        return done(null, false, { message: 'Invalid email or password' });
      }
      
      const isMatch = await user.comparePassword(password);
      if (!isMatch) {
        return done(null, false, { message: 'Invalid email or password' });
      }
      
      return done(null, user);
    } catch (error) {
      return done(error);
    }
  }
));

passport.serializeUser((user, done) => {
  done(null, user._id);
});

passport.deserializeUser(async (id, done) => {
  try {
    const user = await User.findById(id);
    done(null, user);
  } catch (error) {
    done(error);
  }
});

module.exports = passport;
"#;

file_tool.write("config/passport.js", passport_config).await?;
```

#### Phase 5: Route Implementation with Impact Analysis
```rust
// Impact analysis before modifying main app file
let impact_analysis = ImpactAnalysis {
    files_affected: vec!["app.js", "routes/auth.js"],
    potential_breaking_changes: vec![
        "Session middleware configuration might affect existing sessions",
        "New routes might conflict with existing route patterns",
    ],
    mitigation_strategies: vec![
        "Backup original app.js before modification",
        "Use unique route prefixes for auth routes",
        "Test existing functionality after changes",
    ],
};

// Create backup
file_tool.copy("app.js", "app.js.backup").await?;

// Authentication routes
let auth_routes = r#"
const express = require('express');
const passport = require('passport');
const User = require('../models/User');
const router = express.Router();

// Register route
router.post('/register', async (req, res) => {
  try {
    const { username, email, password } = req.body;
    
    // Check if user already exists
    const existingUser = await User.findOne({ 
      $or: [{ email }, { username }] 
    });
    
    if (existingUser) {
      return res.status(400).json({ 
        error: 'User with this email or username already exists' 
      });
    }
    
    const user = new User({ username, email, password });
    await user.save();
    
    res.status(201).json({ 
      message: 'User created successfully',
      user: { id: user._id, username: user.username, email: user.email }
    });
  } catch (error) {
    res.status(500).json({ error: 'Registration failed' });
  }
});

// Login route
router.post('/login', passport.authenticate('local'), (req, res) => {
  res.json({ 
    message: 'Login successful',
    user: { 
      id: req.user._id, 
      username: req.user.username, 
      email: req.user.email 
    }
  });
});

// Logout route
router.post('/logout', (req, res) => {
  req.logout((err) => {
    if (err) {
      return res.status(500).json({ error: 'Logout failed' });
    }
    res.json({ message: 'Logout successful' });
  });
});

module.exports = router;
"#;

file_tool.write("routes/auth.js", auth_routes).await?;
```

### 1.5 After-Action Reflection and Learning

```rust
// Post-execution analysis
let execution_outcome = ExecutionOutcome {
    success: true,
    completion_time: Duration::from_mins(38),
    confidence_accuracy: 0.92, // Actual vs predicted confidence
    errors_encountered: vec![
        "Minor syntax error in user model (quickly fixed)",
        "Package.json formatting issue (auto-resolved)",
    ],
    adaptations_made: vec![
        "Used email as username field instead of separate username",
        "Added backup creation step for safety",
    ],
};

// Learning integration
let patterns_learned = vec![
    Pattern {
        pattern_type: PatternType::SuccessfulImplementation,
        context: "Express.js authentication with Passport",
        conditions: vec![
            "Standard Express app structure",
            "MongoDB/Mongoose for user storage",
            "Passport.js for authentication strategy",
        ],
        success_factors: vec![
            "Progressive validation at each step",
            "Backup creation before major changes",
            "Comprehensive error handling in routes",
        ],
        confidence: 0.95,
    },
];

learning_system.integrate_patterns(patterns_learned).await?;

// Confidence calibration update
confidence_calibrator.update_accuracy(
    predicted_confidence: 0.85,
    actual_outcome: execution_outcome,
);
```

## 2. Error Recovery Example

### Scenario: Database Connection Failure During User Registration

#### 2.1 Error Detection and Classification
```rust
// During user registration implementation
let error = ToolError::DatabaseConnection {
    message: "MongoNetworkError: failed to connect to server",
    error_code: "ECONNREFUSED",
    retry_after: Some(Duration::from_secs(5)),
};

// Error classification
let error_analysis = ErrorAnalyzer::analyze(&error);
// Result: RecoverableError with DatabaseConnectivity category
```

#### 2.2 Hypothesis-Driven Debugging
```rust
// Form hypotheses about the error cause
let hypotheses = vec![
    Hypothesis {
        statement: "MongoDB service is not running",
        test_criteria: vec![
            "Check if mongod process is running",
            "Verify MongoDB service status",
        ],
        confidence: 0.7,
    },
    Hypothesis {
        statement: "Connection string is incorrect",
        test_criteria: vec![
            "Validate MongoDB connection URI format",
            "Check if database name exists",
        ],
        confidence: 0.5,
    },
    Hypothesis {
        statement: "Network connectivity issues",
        test_criteria: vec![
            "Ping MongoDB host",
            "Check firewall rules",
        ],
        confidence: 0.3,
    },
];

// Test hypotheses in order of confidence
for hypothesis in hypotheses {
    let evidence = evidence_collector.collect_for_hypothesis(&hypothesis).await?;
    let validation = hypothesis_validator.validate(&hypothesis, &evidence)?;
    
    if validation.outcome == ValidationOutcome::Confirmed {
        selected_hypothesis = Some(hypothesis);
        break;
    }
}
```

#### 2.3 Adaptive Recovery Strategy
```rust
match selected_hypothesis {
    Some(h) if h.statement.contains("MongoDB service is not running") => {
        // Recovery strategy: start MongoDB service
        let recovery_actions = vec![
            "Check MongoDB installation",
            "Start MongoDB service",
            "Verify service is running",
            "Retry database connection",
        ];
        
        // Execute recovery
        bash_tool.execute("brew services start mongodb/brew/mongodb-community").await?;
        
        // Wait and retry
        tokio::time::sleep(Duration::from_secs(5)).await;
        let retry_result = test_database_connection().await;
        
        if retry_result.is_ok() {
            recovery_manager.record_success("mongodb_service_restart");
        }
    },
    
    Some(h) if h.statement.contains("Connection string is incorrect") => {
        // Recovery strategy: fix connection string
        let current_config = config_manager.get_database_config();
        let suggested_config = DatabaseConfig {
            uri: "mongodb://localhost:27017/myapp".to_string(),
            options: ConnectionOptions::default(),
        };
        
        // Ask user for confirmation or auto-fix if confidence is high
        if meta_reasoner.get_confidence() > 0.8 {
            config_manager.update_database_config(suggested_config).await?;
        }
    },
    
    _ => {
        // Fallback: use alternative approach
        strategy_adapter.switch_to_fallback("sqlite_local_db").await?;
    }
}
```

## 3. Learning and Adaptation Example

### Scenario: Pattern Recognition from Multiple Similar Tasks

#### 3.1 Pattern Extraction
```rust
// After completing several authentication implementations
let completed_tasks = vec![
    CompletedTask {
        description: "Add user auth to Express app",
        approach: "Passport.js + MongoDB",
        outcome: TaskOutcome::Success { duration: Duration::from_mins(38) },
        patterns: vec!["express_passport_pattern", "mongodb_user_schema"],
    },
    CompletedTask {
        description: "Implement JWT authentication", 
        approach: "Custom JWT + PostgreSQL",
        outcome: TaskOutcome::Success { duration: Duration::from_mins(52) },
        patterns: vec!["jwt_custom_pattern", "postgres_user_table"],
    },
    CompletedTask {
        description: "Add OAuth to React app",
        approach: "Auth0 integration",
        outcome: TaskOutcome::PartialSuccess { issues: vec!["CORS configuration"] },
        patterns: vec!["oauth_react_pattern", "cors_auth_issues"],
    },
];

// Extract common patterns
let pattern_extractor = PatternExtractor::new();
let extracted_patterns = pattern_extractor.extract_from_tasks(&completed_tasks);

// Resulting patterns
let auth_implementation_pattern = Pattern {
    name: "authentication_implementation",
    success_conditions: vec![
        "Clear user model/schema definition",
        "Proper password hashing implementation", 
        "Session or token management setup",
        "Protected route middleware",
        "Comprehensive error handling",
    ],
    common_pitfalls: vec![
        "Forgetting to hash passwords",
        "Inadequate session configuration",
        "Missing CORS setup for frontend integration",
        "Insufficient input validation",
    ],
    confidence: 0.92,
    applicability: vec!["express.js", "node.js", "web authentication"],
};
```

#### 3.2 Anti-Pattern Recognition
```rust
// Identify failure patterns from unsuccessful attempts
let anti_patterns = vec![
    AntiPattern {
        name: "cors_misconfiguration_auth",
        description: "Authentication fails due to CORS misconfiguration",
        indicators: vec![
            "Frontend authentication requests fail",
            "CORS-related error messages in browser",
            "Backend authentication works in isolation",
        ],
        prevention_strategies: vec![
            "Configure CORS before authentication middleware",
            "Include credentials: true in CORS config",
            "Test with actual frontend early in implementation",
        ],
        confidence: 0.88,
    },
    AntiPattern {
        name: "session_secret_hardcoded",
        description: "Session secret hardcoded in source code",
        indicators: vec![
            "session({ secret: 'some-string' }) in source",
            "No environment variable for session secret",
        ],
        prevention_strategies: vec![
            "Always use environment variables for secrets",
            "Generate random session secrets",
            "Add security linting rules",
        ],
        confidence: 0.95,
    },
];

// Anti-pattern detection in current code
let code_analyzer = AntiPatternDetector::new(anti_patterns);
let detected_issues = code_analyzer.scan_codebase(&current_implementation);
```

#### 3.3 Confidence Calibration
```rust
// Calibrate confidence based on historical performance
let confidence_calibrator = ConfidenceCalibrator::new();

// Historical data points
let historical_predictions = vec![
    PredictionOutcome {
        predicted_confidence: 0.85,
        predicted_duration: Duration::from_mins(45),
        actual_outcome: TaskOutcome::Success { duration: Duration::from_mins(38) },
        accuracy_score: 0.92,
    },
    PredictionOutcome {
        predicted_confidence: 0.70,
        predicted_duration: Duration::from_mins(60),
        actual_outcome: TaskOutcome::Success { duration: Duration::from_mins(52) },
        accuracy_score: 0.95,
    },
    PredictionOutcome {
        predicted_confidence: 0.90,
        predicted_duration: Duration::from_mins(30),
        actual_outcome: TaskOutcome::PartialSuccess { duration: Duration::from_mins(45) },
        accuracy_score: 0.75,
    },
];

// Update calibration model
confidence_calibrator.update_model(&historical_predictions);

// For new similar tasks, adjust confidence based on learned calibration
let new_task_raw_confidence = 0.80;
let calibrated_confidence = confidence_calibrator.calibrate(
    new_task_raw_confidence,
    &task_features,
);
// Result: 0.75 (slightly lower due to historical overconfidence pattern)
```

## 4. Complex Multi-Step Workflow Example

### Scenario: "Refactor monolithic app into microservices"

#### 4.1 Hierarchical Goal Decomposition
```rust
let primary_goal = Goal {
    description: "Refactor monolithic Express app into microservices architecture",
    success_criteria: vec![
        "Services are properly decoupled",
        "Inter-service communication is established", 
        "Data consistency is maintained",
        "Performance is not degraded",
        "All existing functionality works",
    ],
    estimated_complexity: ComplexityScore::High(9.2),
};

let goal_hierarchy = GoalHierarchy {
    primary_goal,
    sub_goals: vec![
        SubGoal {
            id: "analysis",
            description: "Analyze current monolithic structure",
            tasks: vec![
                "Map current application components",
                "Identify service boundaries",
                "Analyze data dependencies", 
                "Document current API endpoints",
            ],
            dependencies: vec![],
            estimated_duration: Duration::from_hours(2),
        },
        SubGoal {
            id: "service_extraction",
            description: "Extract individual services",
            tasks: vec![
                "Create user service",
                "Create product service", 
                "Create order service",
                "Extract shared utilities",
            ],
            dependencies: vec!["analysis"],
            estimated_duration: Duration::from_hours(6),
        },
        SubGoal {
            id: "communication_setup",
            description: "Setup inter-service communication",
            tasks: vec![
                "Implement API gateway",
                "Setup service discovery",
                "Configure load balancing",
                "Implement circuit breakers",
            ],
            dependencies: vec!["service_extraction"],
            estimated_duration: Duration::from_hours(4),
        },
        SubGoal {
            id: "data_migration",
            description: "Migrate and distribute data",
            tasks: vec![
                "Split monolithic database",
                "Setup data synchronization",
                "Implement event sourcing",
                "Migrate existing data",
            ],
            dependencies: vec!["service_extraction"],
            estimated_duration: Duration::from_hours(8),
        },
        SubGoal {
            id: "testing_deployment",
            description: "Test and deploy services",
            tasks: vec![
                "Setup integration testing",
                "Configure containerization",
                "Implement monitoring",
                "Deploy to staging",
                "Performance testing",
                "Production deployment",
            ],
            dependencies: vec!["communication_setup", "data_migration"],
            estimated_duration: Duration::from_hours(6),
        },
    ],
};
```

#### 4.2 Dynamic Re-planning Example
```rust
// During service extraction phase
let execution_context = ExecutionContext {
    current_sub_goal: "service_extraction",
    completed_tasks: vec!["Map current application components", "Identify service boundaries"],
    current_task: "Create user service",
    discovered_complications: vec![
        "User service has tight coupling with order processing",
        "Shared database transactions across multiple future services",
        "Legacy authentication system tightly integrated",
    ],
};

// Meta-reasoning triggers re-planning
let meta_analysis = meta_reasoner.evaluate_progress(&execution_context);
if meta_analysis.requires_adaptation() {
    let adaptation = strategy_adapter.adapt_plan(&goal_hierarchy, &execution_context);
    
    // Resulting plan modifications
    let updated_sub_goals = vec![
        SubGoal {
            id: "authentication_extraction",
            description: "Extract authentication as separate service first",
            tasks: vec![
                "Create authentication service",
                "Implement JWT token system",
                "Update all services to use central auth",
            ],
            dependencies: vec!["analysis"],
            priority: Priority::High,
        },
        SubGoal {
            id: "database_analysis",
            description: "Detailed database dependency analysis",
            tasks: vec![
                "Map all cross-table dependencies",
                "Design data distribution strategy",
                "Plan transaction boundaries",
            ],
            dependencies: vec!["analysis"],
            priority: Priority::High,
        },
        // Modified service_extraction with updated dependencies
        SubGoal {
            id: "service_extraction",
            dependencies: vec!["authentication_extraction", "database_analysis"],
            // ... rest of sub-goal updated
        },
    ];
}
```

This comprehensive documentation provides concrete examples of how the meta-cognitive reasoning system works in practice, showing the complete flow from task analysis through execution, error recovery, and learning integration.