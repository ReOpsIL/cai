# CAI Test Suite

This directory contains comprehensive tests for the CAI (Chat AI) prompt management application. The tests validate scoring mechanisms, prompt modification/enhancement, and all core functionality.

## Test Structure

### 1. `test_prompt_scoring.rs`
Tests the prompt scoring mechanism:
- **Score increment functionality**: Validates that prompts scores increase correctly
- **Score persistence**: Ensures scores are saved to and loaded from YAML files
- **Error handling**: Tests invalid file/subject/prompt ID scenarios
- **Multiple increments**: Verifies repeated scoring operations
- **Backward compatibility**: Tests loading old YAML files without score fields

### 2. `test_similarity_detection.rs`
Tests the similarity detection algorithm:
- **Text similarity calculation**: Tests Levenshtein distance implementation
- **Threshold testing**: Validates different similarity thresholds (0.8, 0.6, 0.5)
- **Similar prompt finding**: Tests finding prompts with various similarity levels
- **Case sensitivity**: Ensures case-insensitive matching
- **URL content similarity**: Tests similarity detection with file:// references
- **Accuracy validation**: Verifies similarity scores are in expected ranges

### 3. `test_prompt_management.rs`
Tests prompt repository management:
- **Adding new prompts**: Tests adding prompts to existing and new subjects
- **Updating prompt content**: Validates prompt content modification
- **AI-generated file creation**: Tests automatic creation of ai_generated.yaml
- **Error handling**: Tests invalid operations and edge cases
- **Persistence**: Ensures all operations persist to disk correctly

### 4. `test_chat_workflow.rs`
Tests the complete chat workflow simulation:
- **Task processing workflow**: Simulates the three-tier processing (score/update/add)
- **New prompt addition**: Tests adding completely new prompts
- **Prompt scoring workflow**: Tests incrementing scores for similar prompts
- **Prompt update workflow**: Tests updating existing prompts with improvements
- **Task categorization**: Tests automatic categorization into subjects
- **Similarity thresholds**: Tests boundary conditions (0.8, 0.6 thresholds)
- **Multiple task processing**: Tests processing multiple tasks in sequence

### 5. `test_integration.rs`
Integration tests for the complete CLI application:
- **Command-line interface**: Tests all CLI commands (list, search, show, query, chat)
- **Score display**: Validates ⭐ emoji score display in output
- **URL content loading**: Tests file:// URL reference functionality  
- **Error scenarios**: Tests malformed YAML, missing files, invalid directories
- **Custom directories**: Tests --directory parameter
- **Help documentation**: Validates help text and command descriptions

### 6. `test_edge_cases.rs`
Tests edge cases and error conditions:
- **Empty/minimal content**: Tests empty strings, single characters
- **Unicode and special characters**: Tests emoji, accented characters, symbols
- **Very long content**: Tests extremely long prompts and titles
- **Negative/extreme scores**: Tests unusual score values
- **Invalid operations**: Tests operations with malformed inputs
- **File system edge cases**: Tests unusual filenames and scenarios
- **URL edge cases**: Tests invalid URLs and network failures
- **Memory usage**: Tests performance with large datasets

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Module
```bash
cargo test test_prompt_scoring
cargo test test_similarity_detection
cargo test test_prompt_management
cargo test test_chat_workflow
cargo test test_integration
cargo test test_edge_cases
```

### Run Individual Tests
```bash
cargo test test_prompt_score_increment
cargo test test_similarity_calculation
cargo test test_add_prompt_to_existing_subject
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Tests in Release Mode
```bash
cargo test --release
```

## Test Coverage Areas

### ✅ Scoring Mechanisms
- [x] Score increment functionality
- [x] Score persistence across restarts
- [x] Multiple score increments
- [x] Error handling for invalid IDs
- [x] Backward compatibility with scoreless prompts

### ✅ Similarity Detection
- [x] Levenshtein distance calculation
- [x] Threshold-based matching (0.8, 0.6, 0.5)
- [x] Case-insensitive comparison
- [x] URL content resolution and matching
- [x] Accuracy validation across similarity ranges

### ✅ Prompt Modification/Enhancement
- [x] Adding new prompts to existing subjects
- [x] Creating new subjects automatically
- [x] Updating existing prompt content
- [x] Preserving metadata (title, score, ID) during updates
- [x] Error handling for invalid operations

### ✅ Chat Workflow
- [x] Three-tier processing logic (score ≥0.8, update ≥0.6, add <0.6)
- [x] Task categorization into appropriate subjects
- [x] AI-generated file creation and management
- [x] End-to-end workflow simulation
- [x] Boundary condition testing

### ✅ Integration & CLI
- [x] All CLI commands (list, search, show, query, chat)
- [x] Score display formatting
- [x] URL reference handling
- [x] Custom directory support
- [x] Error messaging and help text

### ✅ Edge Cases & Robustness
- [x] Unicode and special character handling
- [x] Empty/minimal content scenarios
- [x] Very long content processing
- [x] Invalid input handling
- [x] File system edge cases
- [x] Network failure resilience

## Mock Objects

The tests use mock objects to avoid external dependencies:

- **MockOpenRouterClient**: Simulates LLM API responses for task planning and prompt improvement
- **MockChatInterface**: Simulates the chat workflow without requiring API keys
- **Temporary directories**: All tests use isolated temporary file systems

## Test Data

Tests create realistic YAML prompt files with:
- Various prompt types (bug fixing, code analysis, task creation)
- Different score values (0-999999)
- Unicode content and special characters
- URL references to local and remote content
- Empty subjects and prompts for edge case testing

## Performance Considerations

The test suite includes performance validation:
- Large dataset handling (1000+ prompts)
- Memory usage monitoring
- Concurrent operation testing
- File I/O performance validation

## Continuous Integration

These tests are designed to run in CI/CD environments:
- No external dependencies (mocked LLM calls)
- Isolated temporary file systems
- Deterministic results
- Comprehensive error reporting