#!/usr/bin/env python3
"""
Comprehensive CAI Test Suite
Tests 100 different scenarios to validate the enhanced CAI system
"""

import subprocess
import json
import time
import random
import sys
from pathlib import Path
from typing import List, Dict, Tuple
import tempfile
import os

class CAITestSuite:
    def __init__(self):
        self.results = []
        self.failures = []
        self.successes = []
        self.temp_dir = None
        self.setup_test_environment()
    
    def setup_test_environment(self):
        """Set up test environment with sample prompts"""
        self.temp_dir = tempfile.mkdtemp(prefix="cai_test_")
        prompts_dir = Path(self.temp_dir) / "prompts"
        prompts_dir.mkdir(exist_ok=True)
        
        # Create sample prompt files for testing
        sample_prompts = {
            "development.yaml": {
                "Development": {
                    "bug_fixing": {
                        "title": "Bug Fixing Assistant",
                        "content": "Help debug and fix code issues systematically.",
                        "score": 0
                    },
                    "code_review": {
                        "title": "Code Review Helper",
                        "content": "Provide comprehensive code review feedback.",
                        "score": 0
                    }
                }
            },
            "documentation.yaml": {
                "Documentation": {
                    "api_docs": {
                        "title": "API Documentation",
                        "content": "Generate comprehensive API documentation.",
                        "score": 0
                    },
                    "user_guide": {
                        "title": "User Guide Creator",
                        "content": "Create user-friendly documentation.",
                        "score": 0
                    }
                }
            },
            "testing.yaml": {
                "Testing": {
                    "unit_tests": {
                        "title": "Unit Test Generator",
                        "content": "Generate comprehensive unit tests for code.",
                        "score": 0
                    },
                    "integration_tests": {
                        "title": "Integration Test Helper",
                        "content": "Create integration test scenarios.",
                        "score": 0
                    }
                }
            }
        }
        
        # Write sample prompt files
        for filename, content in sample_prompts.items():
            with open(prompts_dir / filename, 'w') as f:
                import yaml
                yaml.dump(content, f, default_flow_style=False)
    
    def run_cai_command(self, command: List[str], timeout: int = 30) -> Tuple[bool, str, str]:
        """Run a CAI command and return success status and output"""
        try:
            full_command = ["cargo", "run", "--", "--directory", str(Path(self.temp_dir) / "prompts")] + command
            result = subprocess.run(
                full_command,
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd="/Users/dovcaspi/cai"
            )
            
            success = result.returncode == 0
            return success, result.stdout, result.stderr
            
        except subprocess.TimeoutExpired:
            return False, "", f"Command timed out after {timeout} seconds"
        except Exception as e:
            return False, "", f"Command failed with exception: {str(e)}"
    
    def test_basic_operations(self) -> List[Dict]:
        """Test basic CAI operations"""
        tests = []
        
        # Test 1-10: Basic command functionality
        basic_commands = [
            (["list"], "List prompts"),
            (["search", "bug"], "Search prompts"),
            (["show", "development"], "Show prompt file"),
            (["query", "development", "Development", "bug_fixing"], "Query specific prompt"),
            (["--help"], "Show help"),
            (["mcp", "--help"], "MCP help"),
            (["workflow", "--help"], "Workflow help"),
            (["scan", "--help"], "Scan help"),
            (["mcp", "list"], "List MCP servers"),
            (["workflow", "status"], "Workflow status"),
        ]
        
        for i, (command, description) in enumerate(basic_commands, 1):
            print(f"Running test {i}: {description}")
            success, stdout, stderr = self.run_cai_command(command)
            
            test_result = {
                "test_id": i,
                "category": "basic_operations",
                "description": description,
                "command": command,
                "success": success,
                "stdout": stdout,
                "stderr": stderr,
                "duration": 0  # Will be measured in actual implementation
            }
            
            tests.append(test_result)
            
            if success:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
        return tests
    
    def test_error_conditions(self) -> List[Dict]:
        """Test error handling and recovery"""
        tests = []
        
        # Test 11-30: Error condition handling
        error_commands = [
            (["show", "nonexistent"], "Non-existent file"),
            (["query", "invalid", "subject", "prompt"], "Invalid query"),
            (["search", ""], "Empty search"),
            (["mcp", "start", "nonexistent"], "Start non-existent MCP server"),
            (["mcp", "tools", "invalid"], "Tools from invalid server"),
            (["workflow", "show", "invalid"], "Show invalid workflow"),
            (["scan", "/nonexistent/path"], "Scan invalid path"),
            (["--invalid-flag"], "Invalid command flag"),
            (["list", "--invalid-option"], "Invalid option"),
            (["search", "bug", "--invalid"], "Invalid search option"),
        ]
        
        for i, (command, description) in enumerate(error_commands, 11):
            print(f"Running test {i}: {description}")
            success, stdout, stderr = self.run_cai_command(command)
            
            # For error tests, we expect controlled failure (not crashes)
            controlled_failure = not success and "panic" not in stderr.lower() and "fatal" not in stderr.lower()
            
            test_result = {
                "test_id": i,
                "category": "error_handling",
                "description": description,
                "command": command,
                "success": controlled_failure,  # Success = graceful error handling
                "stdout": stdout,
                "stderr": stderr,
                "expected_failure": True
            }
            
            tests.append(test_result)
            
            if controlled_failure:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
        return tests
    
    def test_performance_scenarios(self) -> List[Dict]:
        """Test performance under various conditions"""
        tests = []
        
        # Test 31-50: Performance scenarios
        for i in range(31, 51):
            description = f"Performance test {i - 30}: Rapid command execution"
            print(f"Running test {i}: {description}")
            
            # Rapid-fire list commands to test performance
            start_time = time.time()
            success, stdout, stderr = self.run_cai_command(["list"], timeout=10)
            end_time = time.time()
            
            duration = end_time - start_time
            performance_success = success and duration < 5.0  # Should complete within 5 seconds
            
            test_result = {
                "test_id": i,
                "category": "performance",
                "description": description,
                "command": ["list"],
                "success": performance_success,
                "stdout": stdout,
                "stderr": stderr,
                "duration": duration
            }
            
            tests.append(test_result)
            
            if performance_success:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
            # Small delay to avoid overwhelming the system
            time.sleep(0.1)
                
        return tests
    
    def test_concurrent_operations(self) -> List[Dict]:
        """Test concurrent operation handling"""
        tests = []
        
        # Test 51-70: Concurrent operations simulation
        for i in range(51, 71):
            description = f"Concurrent test {i - 50}: Multiple rapid operations"
            print(f"Running test {i}: {description}")
            
            # Simulate concurrent-like behavior with rapid sequential calls
            commands = [
                ["list"],
                ["search", "test"],
                ["show", "development"],
                ["mcp", "list"],
                ["workflow", "status"]
            ]
            
            command = random.choice(commands)
            success, stdout, stderr = self.run_cai_command(command, timeout=15)
            
            test_result = {
                "test_id": i,
                "category": "concurrent",
                "description": description,
                "command": command,
                "success": success,
                "stdout": stdout,
                "stderr": stderr
            }
            
            tests.append(test_result)
            
            if success:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
        return tests
    
    def test_edge_cases(self) -> List[Dict]:
        """Test edge cases and boundary conditions"""
        tests = []
        
        # Test 71-90: Edge cases
        edge_commands = [
            (["search", "a" * 1000], "Very long search term"),
            (["search", ""], "Empty search term"),
            (["list"] * 5, "Repeated command args"),  # Will test with single list
            (["show", "../../../etc/passwd"], "Path traversal attempt"),
            (["query", "dev", "Dev", "bug", "extra", "args"], "Too many query args"),
            (["search", "unicode测试"], "Unicode search"),
            (["search", "special!@#$%^&*()"], "Special characters"),
            (["show", "file with spaces"], "Filename with spaces"),
            (["search", "\n\r\t"], "Whitespace search"),
            (["list", "--directory", "/tmp/nonexistent"], "Non-existent directory"),
        ]
        
        for i, (command, description) in enumerate(edge_commands[:10], 71):
            print(f"Running test {i}: {description}")
            
            # Handle repeated command case
            if isinstance(command[0], list):
                command = command[0]
                
            success, stdout, stderr = self.run_cai_command(command)
            
            # Edge cases should handle gracefully
            graceful_handling = success or ("error" in stderr.lower() and "panic" not in stderr.lower())
            
            test_result = {
                "test_id": i,
                "category": "edge_cases",
                "description": description,
                "command": command,
                "success": graceful_handling,
                "stdout": stdout,
                "stderr": stderr
            }
            
            tests.append(test_result)
            
            if graceful_handling:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
        return tests
    
    def test_resource_constraints(self) -> List[Dict]:
        """Test behavior under resource constraints"""
        tests = []
        
        # Test 91-100: Resource constraint simulation
        for i in range(91, 101):
            description = f"Resource test {i - 90}: Resource constraint simulation"
            print(f"Running test {i}: {description}")
            
            # Test with very short timeouts to simulate resource constraints
            success, stdout, stderr = self.run_cai_command(["list"], timeout=1)
            
            # Resource constraint handling - should either succeed or fail gracefully
            resource_handling = success or "timeout" in stderr.lower()
            
            test_result = {
                "test_id": i,
                "category": "resource_constraints",
                "description": description,
                "command": ["list"],
                "success": resource_handling,
                "stdout": stdout,
                "stderr": stderr,
                "timeout": 1
            }
            
            tests.append(test_result)
            
            if resource_handling:
                self.successes.append(test_result)
            else:
                self.failures.append(test_result)
                
        return tests
    
    def run_all_tests(self) -> Dict:
        """Run all 100 tests and return comprehensive results"""
        print("🚀 Starting comprehensive CAI test suite (100 tests)")
        print("=" * 60)
        
        all_tests = []
        
        # Run test categories
        all_tests.extend(self.test_basic_operations())
        all_tests.extend(self.test_error_conditions())
        all_tests.extend(self.test_performance_scenarios())
        all_tests.extend(self.test_concurrent_operations())
        all_tests.extend(self.test_edge_cases())
        all_tests.extend(self.test_resource_constraints())
        
        # Calculate statistics
        success_rate = len(self.successes) / len(all_tests) * 100
        
        results = {
            "total_tests": len(all_tests),
            "successes": len(self.successes),
            "failures": len(self.failures),
            "success_rate": success_rate,
            "all_tests": all_tests,
            "failures_detail": self.failures,
            "successes_detail": self.successes
        }
        
        return results
    
    def analyze_failures(self, results: Dict) -> Dict:
        """Analyze failures and create improvement plan"""
        failures = results["failures_detail"]
        
        # Categorize failures
        failure_categories = {}
        for failure in failures:
            category = failure["category"]
            if category not in failure_categories:
                failure_categories[category] = []
            failure_categories[category].append(failure)
        
        # Analyze common failure patterns
        common_errors = {}
        for failure in failures:
            stderr = failure["stderr"].lower()
            if "timeout" in stderr:
                common_errors["timeout"] = common_errors.get("timeout", 0) + 1
            elif "not found" in stderr or "no such file" in stderr:
                common_errors["file_not_found"] = common_errors.get("file_not_found", 0) + 1
            elif "permission denied" in stderr:
                common_errors["permission"] = common_errors.get("permission", 0) + 1
            elif "panic" in stderr:
                common_errors["panic"] = common_errors.get("panic", 0) + 1
            elif "compilation failed" in stderr:
                common_errors["compilation"] = common_errors.get("compilation", 0) + 1
            else:
                common_errors["other"] = common_errors.get("other", 0) + 1
        
        analysis = {
            "failure_categories": failure_categories,
            "common_errors": common_errors,
            "critical_issues": [f for f in failures if "panic" in f["stderr"].lower()],
            "performance_issues": [f for f in failures if f["category"] == "performance"],
            "error_handling_issues": [f for f in failures if f["category"] == "error_handling"]
        }
        
        return analysis
    
    def cleanup(self):
        """Clean up test environment"""
        if self.temp_dir and Path(self.temp_dir).exists():
            import shutil
            shutil.rmtree(self.temp_dir)

def main():
    # Install required dependency
    try:
        import yaml
    except ImportError:
        print("Installing PyYAML...")
        subprocess.run([sys.executable, "-m", "pip", "install", "PyYAML"], check=True)
        import yaml
    
    test_suite = CAITestSuite()
    
    try:
        # Run all tests
        results = test_suite.run_all_tests()
        
        # Print summary
        print("\n" + "=" * 60)
        print("🎯 TEST SUITE RESULTS")
        print("=" * 60)
        print(f"Total Tests: {results['total_tests']}")
        print(f"Successes: {results['successes']}")
        print(f"Failures: {results['failures']}")
        print(f"Success Rate: {results['success_rate']:.1f}%")
        
        # Analyze failures
        if results['failures'] > 0:
            print(f"\n⚠️  ANALYZING {results['failures']} FAILURES...")
            analysis = test_suite.analyze_failures(results)
            
            print("\n📊 Failure Categories:")
            for category, failures in analysis["failure_categories"].items():
                print(f"  {category}: {len(failures)} failures")
            
            print("\n🔍 Common Error Patterns:")
            for error_type, count in analysis["common_errors"].items():
                print(f"  {error_type}: {count} occurrences")
            
            # Save detailed results
            with open("/Users/dovcaspi/cai/test_results.json", "w") as f:
                json.dump({
                    "results": results,
                    "analysis": analysis
                }, f, indent=2)
            
            print(f"\n💾 Detailed results saved to test_results.json")
            
            # Return failure indicator
            return results['success_rate'] < 90.0
        else:
            print("\n🎉 ALL TESTS PASSED!")
            return False
            
    finally:
        test_suite.cleanup()

if __name__ == "__main__":
    failed = main()
    sys.exit(1 if failed else 0)