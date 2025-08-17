#!/usr/bin/env python3
"""
CAI Test Runner - Executes the 100 diverse prompts test suite
"""

import subprocess
import json
import yaml
import time
import os
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any, Optional, Tuple
import tempfile
import shutil

class TestResult:
    def __init__(self, test_id: str, category: str, command: str, expected_behavior: str):
        self.test_id = test_id
        self.category = category
        self.command = command
        self.expected_behavior = expected_behavior
        self.success = False
        self.stdout = ""
        self.stderr = ""
        self.return_code = 0
        self.execution_time = 0.0
        self.error_message = ""
        self.timestamp = datetime.now().isoformat()

    def to_dict(self) -> Dict[str, Any]:
        return {
            'test_id': self.test_id,
            'category': self.category,
            'command': self.command,
            'expected_behavior': self.expected_behavior,
            'success': self.success,
            'stdout': self.stdout,
            'stderr': self.stderr,
            'return_code': self.return_code,
            'execution_time': self.execution_time,
            'error_message': self.error_message,
            'timestamp': self.timestamp
        }

class TestRunner:
    def __init__(self, test_suite_file: str, output_dir: str):
        self.test_suite_file = test_suite_file
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        # Load test suite
        with open(test_suite_file, 'r') as f:
            self.test_suite = yaml.safe_load(f)
        
        self.results: List[TestResult] = []
        self.start_time = None
        self.end_time = None
        
        # Setup environment
        self.setup_environment()
    
    def setup_environment(self):
        """Setup test environment"""
        print("🔧 Setting up test environment...")
        
        # Ensure CAI is built
        print("📦 Building CAI...")
        try:
            subprocess.run(["cargo", "build", "--release"], 
                         cwd="/Users/dovcaspi/cai", 
                         check=True, 
                         capture_output=True)
            print("✅ CAI built successfully")
        except subprocess.CalledProcessError as e:
            print(f"❌ Failed to build CAI: {e}")
            print(f"stderr: {e.stderr.decode()}")
            sys.exit(1)
        
        # Create test directories and files
        self.create_test_fixtures()
    
    def create_test_fixtures(self):
        """Create test fixtures needed for tests"""
        test_fixtures_dir = self.output_dir / "fixtures"
        test_fixtures_dir.mkdir(exist_ok=True)
        
        # Create test prompt files
        prompts_dir = test_fixtures_dir / "prompts"
        prompts_dir.mkdir(exist_ok=True)
        
        # Basic prompt file
        basic_prompt = {
            'name': 'Test Basic Prompts',
            'description': 'Basic prompts for testing',
            'subjects': [
                {
                    'name': 'debugging',
                    'prompts': [
                        {
                            'title': 'trace error',
                            'content': 'Help me trace and fix this error in my code.',
                            'score': 5
                        }
                    ]
                }
            ]
        }
        
        with open(prompts_dir / "bug_fixing.yaml", 'w') as f:
            yaml.dump(basic_prompt, f)
        
        # URL reference prompt file
        url_prompt = {
            'name': 'URL References',
            'description': 'Prompts with URL references',
            'subjects': [
                {
                    'name': 'examples',
                    'prompts': [
                        {
                            'title': 'external reference',
                            'content': 'file://./README.md',
                            'score': 3
                        }
                    ]
                }
            ]
        }
        
        with open(prompts_dir / "url_references.yaml", 'w') as f:
            yaml.dump(url_prompt, f)
        
        # Empty file for testing
        (prompts_dir / "empty_file.yaml").touch()
        
        # Large file for testing
        large_content = {
            'name': 'Large Test File',
            'description': 'A large file for performance testing',
            'subjects': []
        }
        
        # Add many subjects to make it large
        for i in range(100):
            subject = {
                'name': f'subject_{i}',
                'prompts': [
                    {
                        'title': f'prompt_{j}',
                        'content': f'This is test prompt {j} in subject {i}. ' * 50,
                        'score': j % 6
                    } for j in range(10)
                ]
            }
            large_content['subjects'].append(subject)
        
        with open(prompts_dir / "large_file.yaml", 'w') as f:
            yaml.dump(large_content, f)
        
        # Malformed YAML for error testing
        with open(prompts_dir / "malformed_yaml.yaml", 'w') as f:
            f.write("invalid: yaml: content: [\n unclosed bracket")
        
        # Special characters file
        special_chars_content = {
            'name': '特殊字符测试 Special Characters Test',
            'description': 'Testing Unicode and special characters: éñ中文🚀',
            'subjects': [
                {
                    'name': 'unicode_test',
                    'prompts': [
                        {
                            'title': '🚀 émojî tëst',
                            'content': 'Testing Unicode: 中文日本語العربية русский',
                            'score': 4
                        }
                    ]
                }
            ]
        }
        
        with open(prompts_dir / "special_chars.yaml", 'w') as f:
            yaml.dump(special_chars_content, f, allow_unicode=True)
        
        # Create test directories
        (test_fixtures_dir / "empty_test_dir").mkdir(exist_ok=True)
        
        nested_prompts_dir = test_fixtures_dir / "nested_prompts"
        nested_prompts_dir.mkdir(exist_ok=True)
        nested_sub_dir = nested_prompts_dir / "subdirectory"
        nested_sub_dir.mkdir(exist_ok=True)
        
        with open(nested_sub_dir / "nested_prompt.yaml", 'w') as f:
            yaml.dump({
                'name': 'Nested Prompt',
                'description': 'A prompt in a subdirectory',
                'subjects': [{'name': 'test', 'prompts': [{'title': 'nested', 'content': 'nested content'}]}]
            }, f)
        
        print(f"✅ Test fixtures created in {test_fixtures_dir}")
    
    def fix_cli_flag_positioning(self, command: str) -> str:
        """Fix CLI flag positioning - move global flags before subcommands"""
        parts = command.split()
        if len(parts) < 2:
            return command
            
        # Global flags that should come before subcommands
        global_flags = {'--directory', '--mode', '--debug'}
        
        cai_idx = -1
        subcommand_idx = -1
        
        # Find CAI binary and subcommand positions
        for i, part in enumerate(parts):
            if 'cai' in part:
                cai_idx = i
            elif cai_idx != -1 and not part.startswith('-') and subcommand_idx == -1:
                subcommand_idx = i
                break
        
        if cai_idx == -1 or subcommand_idx == -1:
            return command
            
        # Extract flags that come after the subcommand
        flags_to_move = []
        remaining_parts = []
        
        for i in range(subcommand_idx + 1, len(parts)):
            part = parts[i]
            if part in global_flags:
                flags_to_move.append(part)
                # Add the flag value if next part doesn't start with '-'
                if i + 1 < len(parts) and not parts[i + 1].startswith('-'):
                    flags_to_move.append(parts[i + 1])
                    i += 1  # Skip the value part
            elif not flags_to_move or not parts[i-1] in global_flags:
                remaining_parts.append(part)
        
        # Reconstruct command with proper flag positioning
        if flags_to_move:
            new_parts = (
                parts[:cai_idx + 1] +  # CAI binary
                flags_to_move +        # Global flags
                parts[subcommand_idx:subcommand_idx + 1] +  # Subcommand
                remaining_parts        # Remaining arguments
            )
            return ' '.join(new_parts)
        
        return command
    
    def run_test(self, test_config: Dict[str, Any]) -> TestResult:
        """Run a single test"""
        result = TestResult(
            test_config['id'],
            test_config['category'],
            test_config['command'],
            test_config['expected_behavior']
        )
        
        print(f"🧪 Running {result.test_id}: {result.command}")
        
        # Prepare command
        command = result.command
        
        # Handle special environment variables and command modifications
        env = os.environ.copy()
        shell_command = False
        
        if "NO_COLOR=1" in command:
            env["NO_COLOR"] = "1"
            command = command.replace("NO_COLOR=1 ", "")
        
        if "CAI_LOG_LEVEL=debug" in command:
            env["CAI_LOG_LEVEL"] = "debug"
            command = command.replace("CAI_LOG_LEVEL=debug ", "")
        
        if "unset OPENROUTER_API_KEY" in command:
            if "OPENROUTER_API_KEY" in env:
                del env["OPENROUTER_API_KEY"]
            command = command.split(" && ", 1)[-1]  # Take part after &&
        
        if "OPENROUTER_API_KEY=invalid" in command:
            env["OPENROUTER_API_KEY"] = "invalid"
            command = command.replace("OPENROUTER_API_KEY=invalid ", "")
        
        # Always use shell for commands with pipes, redirects, complex syntax
        if ("|" in command or "&&" in command or "timeout " in command or 
            "time " in command or " & " in command or "echo " in command or
            "for " in command or "ln -s" in command or "mkdir -p" in command or
            "env -i" in command or "LC_ALL=" in command or "/usr/bin/time" in command):
            shell_command = True
        
        # Replace directory placeholders with actual test fixture paths
        fixtures_dir = self.output_dir / "fixtures"
        command = command.replace("./prompts", str(fixtures_dir / "prompts"))
        command = command.replace("/tmp/empty_test_dir", str(fixtures_dir / "empty_test_dir"))
        command = command.replace("./nested_prompts", str(fixtures_dir / "nested_prompts"))
        
        # Handle environment variables at the start of commands
        env_vars = {}
        command_parts = command.split()
        command_start_idx = 0
        
        # Extract environment variables from command prefix
        for i, part in enumerate(command_parts):
            if '=' in part and not part.startswith('-'):
                var_name, var_value = part.split('=', 1)
                env_vars[var_name] = var_value
                command_start_idx = i + 1
            else:
                break
        
        # Rebuild command without environment variables
        if env_vars:
            command = ' '.join(command_parts[command_start_idx:])
            # Update the environment
            for var_name, var_value in env_vars.items():
                env[var_name] = var_value
        
        # Fix CLI flag positioning for global flags
        command = self.fix_cli_flag_positioning(command)
        
        # Replace 'cai' with full path to binary for all commands
        command = command.replace(" cai ", " ./target/release/cai ")
        command = command.replace("| cai ", "| ./target/release/cai ")
        if command.startswith("cai "):
            command = "./target/release/cai " + command[4:]
        
        # Handle directory change to CAI project
        original_cwd = os.getcwd()
        os.chdir("/Users/dovcaspi/cai")
        
        try:
            start_time = time.time()
            
            if shell_command:
                # Use shell for complex commands
                process = subprocess.run(
                    command,
                    shell=True,
                    capture_output=True,
                    text=True,
                    timeout=30,  # 30 second timeout
                    env=env
                )
            else:
                # Split command for direct execution
                cmd_parts = command.split()
                
                # Replace 'cai' with actual binary path
                if cmd_parts[0] == "cai":
                    cmd_parts[0] = "./target/release/cai"
                
                process = subprocess.run(
                    cmd_parts,
                    capture_output=True,
                    text=True,
                    timeout=30,  # 30 second timeout
                    env=env
                )
            
            end_time = time.time()
            result.execution_time = end_time - start_time
            result.stdout = process.stdout
            result.stderr = process.stderr
            result.return_code = process.returncode
            
            # Determine success based on expected behavior and return code
            result.success = self.evaluate_test_success(result)
            
        except subprocess.TimeoutExpired:
            result.error_message = "Test timed out after 30 seconds"
            result.return_code = -1
            result.success = False
        except FileNotFoundError as e:
            result.error_message = f"Command not found: {e}"
            result.return_code = -1
            result.success = False
        except Exception as e:
            result.error_message = f"Unexpected error: {e}"
            result.return_code = -1
            result.success = False
        finally:
            os.chdir(original_cwd)
        
        return result
    
    def evaluate_test_success(self, result: TestResult) -> bool:
        """Evaluate if a test was successful based on expected behavior"""
        expected = result.expected_behavior.lower()
        stdout_lower = result.stdout.lower()
        stderr_lower = result.stderr.lower()
        command_lower = result.command.lower()
        
        # Handle commands that should fail (invalid commands, bad paths, etc.)
        if ("invalid" in command_lower or 
            "non/existent" in command_lower or
            "invalid_command" in command_lower):
            # These should return non-zero exit codes
            return result.return_code != 0 and ("error" in stderr_lower or "unrecognized" in stderr_lower)
        
        # Handle expected error scenarios
        if ("should handle" in expected and "error" in expected) or "should fail" in expected:
            return result.return_code != 0 or "error" in stderr_lower
        
        if "should show" in expected and "error" in expected:
            return "error" in stdout_lower or "error" in stderr_lower
        
        if "not found" in expected or "should not find" in expected:
            return "not found" in stdout_lower or "not found" in stderr_lower or result.return_code != 0
        
        # Handle expected directory errors
        if "/non/existent" in command_lower or "does not exist" in expected:
            return result.return_code != 0
        
        # Handle help and version commands
        if "--help" in command_lower:
            return result.return_code == 0 and ("usage:" in stdout_lower or "help" in stdout_lower)
        
        if "--version" in command_lower:
            return result.return_code == 0 and len(result.stdout.strip()) > 0
        
        # Basic success indicators for normal operations
        if "should display" in expected or "should show" in expected or "should list" in expected:
            return result.return_code == 0 and len(result.stdout) > 0
        
        if "should create" in expected:
            return result.return_code == 0
        
        if "should start" in expected or "should initialize" in expected:
            return result.return_code == 0
        
        # Default: success if return code is 0 and no critical errors
        if result.return_code == 0:
            return True
        
        # For non-zero return codes, check if this might be expected behavior
        return False
    
    def run_all_tests(self) -> Dict[str, Any]:
        """Run all tests in the test suite"""
        print(f"🚀 Starting test suite: {self.test_suite['name']}")
        print(f"📊 Total tests: {len(self.test_suite['tests'])}")
        
        self.start_time = datetime.now()
        
        for i, test_config in enumerate(self.test_suite['tests'], 1):
            print(f"\n[{i}/{len(self.test_suite['tests'])}] ", end="")
            result = self.run_test(test_config)
            self.results.append(result)
            
            status = "✅ PASS" if result.success else "❌ FAIL"
            print(f"{status} ({result.execution_time:.2f}s)")
            
            if not result.success and result.error_message:
                print(f"   Error: {result.error_message}")
            
            # Save individual test result
            self.save_individual_result(result)
        
        self.end_time = datetime.now()
        
        # Generate summary report
        summary = self.generate_summary()
        self.save_summary(summary)
        
        return summary
    
    def save_individual_result(self, result: TestResult):
        """Save individual test result"""
        result_file = self.output_dir / f"{result.test_id}_result.json"
        with open(result_file, 'w') as f:
            json.dump(result.to_dict(), f, indent=2)
    
    def generate_summary(self) -> Dict[str, Any]:
        """Generate test summary"""
        total_tests = len(self.results)
        passed_tests = sum(1 for r in self.results if r.success)
        failed_tests = total_tests - passed_tests
        
        # Category breakdown
        category_stats = {}
        for result in self.results:
            if result.category not in category_stats:
                category_stats[result.category] = {'total': 0, 'passed': 0, 'failed': 0}
            
            category_stats[result.category]['total'] += 1
            if result.success:
                category_stats[result.category]['passed'] += 1
            else:
                category_stats[result.category]['failed'] += 1
        
        # Failed tests details
        failed_test_details = [
            {
                'test_id': r.test_id,
                'category': r.category,
                'command': r.command,
                'error': r.error_message or r.stderr,
                'return_code': r.return_code
            }
            for r in self.results if not r.success
        ]
        
        # Performance stats
        execution_times = [r.execution_time for r in self.results]
        avg_execution_time = sum(execution_times) / len(execution_times) if execution_times else 0
        max_execution_time = max(execution_times) if execution_times else 0
        min_execution_time = min(execution_times) if execution_times else 0
        
        total_duration = (self.end_time - self.start_time).total_seconds() if self.start_time and self.end_time else 0
        
        summary = {
            'test_suite': self.test_suite['name'],
            'execution_info': {
                'start_time': self.start_time.isoformat() if self.start_time else None,
                'end_time': self.end_time.isoformat() if self.end_time else None,
                'total_duration_seconds': total_duration
            },
            'statistics': {
                'total_tests': total_tests,
                'passed_tests': passed_tests,
                'failed_tests': failed_tests,
                'success_rate': (passed_tests / total_tests * 100) if total_tests > 0 else 0,
                'avg_execution_time': avg_execution_time,
                'max_execution_time': max_execution_time,
                'min_execution_time': min_execution_time
            },
            'category_breakdown': category_stats,
            'failed_tests': failed_test_details,
            'all_results': [r.to_dict() for r in self.results]
        }
        
        return summary
    
    def save_summary(self, summary: Dict[str, Any]):
        """Save test summary"""
        summary_file = self.output_dir / "test_summary.json"
        with open(summary_file, 'w') as f:
            json.dump(summary, f, indent=2)
        
        # Also create a human-readable report
        self.create_human_readable_report(summary)
    
    def create_human_readable_report(self, summary: Dict[str, Any]):
        """Create human-readable test report"""
        report_file = self.output_dir / "test_report.md"
        
        with open(report_file, 'w') as f:
            f.write(f"# {summary['test_suite']} - Test Report\n\n")
            
            # Executive Summary
            f.write("## Executive Summary\n\n")
            stats = summary['statistics']
            f.write(f"- **Total Tests**: {stats['total_tests']}\n")
            f.write(f"- **Passed**: {stats['passed_tests']} ({stats['success_rate']:.1f}%)\n")
            f.write(f"- **Failed**: {stats['failed_tests']}\n")
            f.write(f"- **Execution Time**: {summary['execution_info']['total_duration_seconds']:.1f} seconds\n")
            f.write(f"- **Average Test Time**: {stats['avg_execution_time']:.2f} seconds\n\n")
            
            # Category Breakdown
            f.write("## Category Breakdown\n\n")
            for category, cat_stats in summary['category_breakdown'].items():
                success_rate = (cat_stats['passed'] / cat_stats['total'] * 100) if cat_stats['total'] > 0 else 0
                f.write(f"- **{category}**: {cat_stats['passed']}/{cat_stats['total']} ({success_rate:.1f}%)\n")
            f.write("\n")
            
            # Failed Tests
            if summary['failed_tests']:
                f.write("## Failed Tests\n\n")
                for failed in summary['failed_tests']:
                    f.write(f"### {failed['test_id']} ({failed['category']})\n\n")
                    f.write(f"**Command**: `{failed['command']}`\n\n")
                    f.write(f"**Return Code**: {failed['return_code']}\n\n")
                    if failed['error']:
                        f.write(f"**Error**:\n```\n{failed['error']}\n```\n\n")
            
            # Recommendations
            f.write("## Recommendations\n\n")
            if stats['success_rate'] < 50:
                f.write("- 🚨 **Critical**: Success rate below 50%. Major functionality issues need immediate attention.\n")
            elif stats['success_rate'] < 80:
                f.write("- ⚠️ **Warning**: Success rate below 80%. Several functionality gaps need addressing.\n")
            elif stats['success_rate'] < 95:
                f.write("- 💛 **Good**: Success rate above 80%. Some edge cases and error handling need improvement.\n")
            else:
                f.write("- ✅ **Excellent**: Success rate above 95%. System is functioning well.\n")
            
            f.write("\n")
            
            # Next Steps
            f.write("## Next Steps\n\n")
            f.write("1. **Fix Critical Issues**: Address failed tests in order of importance\n")
            f.write("2. **Improve Error Handling**: Enhance error messages and recovery mechanisms\n")
            f.write("3. **Performance Optimization**: Optimize slow-running tests\n")
            f.write("4. **Feature Completion**: Implement missing functionality\n")
            f.write("5. **Re-run Tests**: Validate fixes by re-running failed tests\n\n")

def main():
    if len(sys.argv) != 2:
        print("Usage: python test_runner.py <output_directory>")
        sys.exit(1)
    
    output_dir = sys.argv[1]
    test_suite_file = "/Users/dovcaspi/cai/test_suite_100_prompts.yaml"
    
    runner = TestRunner(test_suite_file, output_dir)
    summary = runner.run_all_tests()
    
    print(f"\n🎯 Test Suite Complete!")
    print(f"📊 Results: {summary['statistics']['passed_tests']}/{summary['statistics']['total_tests']} passed ({summary['statistics']['success_rate']:.1f}%)")
    print(f"📁 Detailed results saved to: {output_dir}")
    print(f"📄 Human-readable report: {output_dir}/test_report.md")

if __name__ == "__main__":
    main()