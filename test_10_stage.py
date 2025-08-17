#!/usr/bin/env python3
"""
10-Stage Web Application Test Suite for CAI
Tests multi-stage development capabilities with incremental complexity
"""

import subprocess
import json
import time
import os
from typing import List, Dict, Any

class CAITestRunner:
    def __init__(self):
        self.test_results = []
        self.project_dir = "/tmp/TaskManager"
        self.cai_binary = "./target/debug/cai"
        
    def setup_test_environment(self):
        """Set up clean test environment"""
        # Remove existing test project
        if os.path.exists(self.project_dir):
            subprocess.run(['rm', '-rf', self.project_dir], check=True)
        
        # Create test directory
        os.makedirs(self.project_dir, exist_ok=True)
        os.chdir(self.project_dir)
        print(f"🏗️ Test environment set up at: {self.project_dir}")
    
    def run_cai_task(self, prompt: str, stage: int) -> Dict[str, Any]:
        """Run a single CAI task and capture results"""
        print(f"\n🔄 Stage {stage}: Testing prompt...")
        print(f"📝 Prompt: {prompt[:80]}...")
        
        start_time = time.time()
        
        try:
            # Use echo to pipe the prompt to CAI chat
            process = subprocess.run([
                'bash', '-c', 
                f'echo "{prompt}" | timeout 60s {self.cai_binary} chat'
            ], 
            capture_output=True, 
            text=True, 
            timeout=70,
            cwd=self.project_dir
            )
            
            end_time = time.time()
            execution_time = end_time - start_time
            
            result = {
                'stage': stage,
                'prompt': prompt,
                'success': process.returncode == 0,
                'stdout': process.stdout,
                'stderr': process.stderr,
                'execution_time': execution_time,
                'files_created': self.scan_project_files(),
                'context_retained': self.check_context_retention(stage)
            }
            
            # Check for specific success indicators
            result['task_completed'] = self.verify_stage_completion(stage, result)
            
            return result
            
        except subprocess.TimeoutExpired:
            return {
                'stage': stage,
                'prompt': prompt,
                'success': False,
                'error': 'Timeout after 60 seconds',
                'execution_time': 60.0,
                'files_created': [],
                'context_retained': False,
                'task_completed': False
            }
        except Exception as e:
            return {
                'stage': stage,
                'prompt': prompt,
                'success': False,
                'error': str(e),
                'execution_time': 0,
                'files_created': [],
                'context_retained': False,
                'task_completed': False
            }
    
    def scan_project_files(self) -> List[str]:
        """Scan current project directory for created files"""
        files = []
        try:
            for root, dirs, filenames in os.walk('.'):
                for filename in filenames:
                    files.append(os.path.join(root, filename))
        except Exception:
            pass
        return files
    
    def check_context_retention(self, stage: int) -> bool:
        """Check if context from previous stages is retained"""
        if stage == 1:
            return True
        
        # Check for presence of files from previous stages
        expected_patterns = {
            2: ['package.json', 'src/', 'App'],
            3: ['router', 'routes'],
            4: ['redux', 'store', 'slice'],
            5: ['Task', 'component'],
            6: ['TaskList', 'filter'],
            7: ['TaskForm', 'validation'],
            8: ['api', 'axios', 'crud'],
            9: ['css', 'style', 'tailwind'],
            10: ['test', 'jest', 'testing']
        }
        
        if stage in expected_patterns:
            files_content = ' '.join(self.scan_project_files()).lower()
            return any(pattern.lower() in files_content for pattern in expected_patterns[stage])
        
        return False
    
    def verify_stage_completion(self, stage: int, result: Dict[str, Any]) -> bool:
        """Verify if the stage was completed successfully"""
        files = result.get('files_created', [])
        files_str = ' '.join(files).lower()
        stdout = result.get('stdout', '').lower()
        
        # Stage-specific completion checks
        completion_checks = {
            1: lambda: 'package.json' in files_str or 'react' in stdout,
            2: lambda: 'router' in files_str or 'routes' in stdout,
            3: lambda: 'redux' in files_str or 'store' in stdout,
            4: lambda: 'task' in files_str or 'component' in stdout,
            5: lambda: 'tasklist' in files_str or 'filter' in stdout,
            6: lambda: 'form' in files_str or 'validation' in stdout,
            7: lambda: 'api' in files_str or 'axios' in stdout,
            8: lambda: 'css' in files_str or 'style' in stdout,
            9: lambda: 'test' in files_str or 'jest' in stdout,
            10: lambda: 'build' in files_str or 'deploy' in stdout
        }
        
        if stage in completion_checks:
            return completion_checks[stage]()
        
        return result['success']
    
    def run_full_test_suite(self):
        """Run the complete 10-stage test suite"""
        
        test_prompts = [
            "Create a new React project called 'TaskManager' with TypeScript, set up the basic folder structure, and create a simple Hello World component.",
            "Add React Router to the TaskManager project and create basic routes for Home, Tasks, and Settings pages.",
            "Implement Redux Toolkit for state management and create slices for tasks and user preferences.",
            "Create a Task component that displays individual tasks with title, description, priority, and completion status.",
            "Build a TaskList component that renders multiple Task components and includes filtering by priority and completion status.",
            "Create a TaskForm component for adding and editing tasks with validation and proper form handling.",
            "Add API integration using axios to connect to a REST API for CRUD operations on tasks.",
            "Implement responsive CSS styling using Tailwind CSS or styled-components to make the application visually appealing.",
            "Add unit tests for the Task and TaskList components using Jest and React Testing Library.",
            "Configure the project for deployment and create build scripts for production deployment."
        ]
        
        print("🚀 Starting 10-Stage CAI Test Suite")
        print("=" * 60)
        
        self.setup_test_environment()
        
        for i, prompt in enumerate(test_prompts, 1):
            result = self.run_cai_task(prompt, i)
            self.test_results.append(result)
            
            # Print immediate feedback
            status = "✅ PASS" if result['task_completed'] else "❌ FAIL"
            print(f"{status} Stage {i}: {result['execution_time']:.1f}s")
            
            if not result['success']:
                print(f"   Error: {result.get('error', 'Unknown error')}")
            
            # Small delay between stages
            time.sleep(1)
        
        self.generate_report()
        return self.test_results
    
    def generate_report(self):
        """Generate comprehensive test report"""
        print("\n" + "=" * 60)
        print("📊 CAI 10-Stage Test Report")
        print("=" * 60)
        
        passed = sum(1 for r in self.test_results if r['task_completed'])
        total = len(self.test_results)
        
        print(f"Overall Success Rate: {passed}/{total} ({passed/total*100:.1f}%)")
        print(f"Average Execution Time: {sum(r['execution_time'] for r in self.test_results)/total:.1f}s")
        
        print("\nStage-by-Stage Results:")
        print("-" * 40)
        
        for result in self.test_results:
            status = "✅ PASS" if result['task_completed'] else "❌ FAIL"
            context = "🔗 YES" if result['context_retained'] else "🔗 NO"
            
            print(f"Stage {result['stage']:2d}: {status} | Context: {context} | {result['execution_time']:.1f}s")
            
            if not result['task_completed']:
                print(f"         Issue: {result.get('error', 'Task incomplete')}")
        
        print("\nFiles Created:")
        print("-" * 40)
        all_files = set()
        for result in self.test_results:
            all_files.update(result.get('files_created', []))
        
        for file in sorted(all_files):
            print(f"  {file}")
        
        # Save detailed results to JSON
        with open('/tmp/cai_test_results.json', 'w') as f:
            json.dump(self.test_results, f, indent=2)
        
        print(f"\n📁 Detailed results saved to: /tmp/cai_test_results.json")

if __name__ == "__main__":
    runner = CAITestRunner()
    results = runner.run_full_test_suite()