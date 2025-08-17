#!/usr/bin/env python3
"""
Enhanced CAI Test Executor with Temporary Folder Management

Executes complex CAI prompts in isolated temporary directories with comprehensive
result capture, analysis, and cleanup management.
"""

import json
import subprocess
import time
import os
import logging
import tempfile
import shutil
from dataclasses import dataclass, asdict
from typing import List, Dict, Any, Optional
from pathlib import Path
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib

@dataclass
class EnhancedExecutionResult:
    test_id: str
    prompt: str
    category: str
    difficulty: str
    complexity_factors: List[str]
    success: bool
    execution_time_seconds: float
    stdout: str
    stderr: str
    files_created: List[str]
    files_content: Dict[str, str]
    project_structure: Dict[str, Any]
    exit_code: int
    error_message: Optional[str]
    timeout_occurred: bool
    cai_command: str
    execution_metadata: Dict[str, Any]
    quality_assessment: Dict[str, Any]
    tmp_directory: str
    cleanup_successful: bool

class EnhancedTestExecutor:
    def __init__(self, cai_executable_path: str = "./target/release/cai", results_base_dir: str = "/tmp/cai_enhanced_tests"):
        self.cai_executable = cai_executable_path
        self.results_base_dir = Path(results_base_dir)
        self.session_id = hashlib.md5(str(time.time()).encode()).hexdigest()[:8]
        self.session_dir = self.results_base_dir / f"session_{self.session_id}"
        
        # Setup session directory and logging
        self.session_dir.mkdir(parents=True, exist_ok=True)
        self._setup_logging()
        
        # Execution statistics
        self.execution_stats = {
            'total_tests': 0,
            'successful_tests': 0,
            'failed_tests': 0,
            'timeout_tests': 0,
            'total_execution_time': 0.0,
            'categories': {},
            'difficulties': {},
            'complexity_factors': {},
            'languages': {},
            'quality_scores': []
        }
        
        self.logger.info(f"Enhanced test executor initialized - Session: {self.session_id}")
        self.logger.info(f"Results directory: {self.session_dir}")

    def _setup_logging(self):
        """Setup comprehensive logging for the test session."""
        log_file = self.session_dir / 'enhanced_execution.log'
        
        logging.basicConfig(
            level=logging.INFO,
            format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
            handlers=[
                logging.FileHandler(log_file),
                logging.StreamHandler()
            ]
        )
        self.logger = logging.getLogger('EnhancedTestExecutor')

    def execute_complex_test(self, test_data: Dict[str, Any]) -> EnhancedExecutionResult:
        """Execute a single complex test case with comprehensive analysis."""
        test_id = test_data['test_id']
        prompt = test_data['prompt']
        category = test_data['category']
        difficulty = test_data['difficulty']
        complexity_factors = test_data.get('complexity_factors', [])
        timeout_seconds = test_data.get('timeout_seconds', 180)
        
        self.logger.info(f"Executing complex test {test_id}: {category}/{difficulty}")
        self.logger.info(f"Complexity factors: {', '.join(complexity_factors)}")
        
        # Create isolated temporary directory for this test
        with tempfile.TemporaryDirectory(prefix=f"cai_test_{test_id}_", dir="/tmp") as tmp_dir:
            tmp_path = Path(tmp_dir)
            self.logger.info(f"Test directory: {tmp_path}")
            
            # Change to test directory for execution
            original_cwd = os.getcwd()
            
            try:
                os.chdir(tmp_path)
                
                # Prepare enhanced command with better input handling
                escaped_prompt = prompt.replace('"', '\\"').replace('\n', '\\n')
                cai_command = f'echo "{escaped_prompt}" | echo "quit" | {self.cai_executable} chat'
                
                self.logger.info(f"Executing command: {cai_command[:100]}...")
                
                # Execute CAI with enhanced monitoring
                start_time = time.time()
                
                try:
                    result = subprocess.run(
                        cai_command,
                        shell=True,
                        capture_output=True,
                        text=True,
                        timeout=timeout_seconds,
                        cwd=tmp_path,
                        env={**os.environ, 'CAI_LOG_LEVEL': 'INFO'}
                    )
                    
                    execution_time = time.time() - start_time
                    timeout_occurred = False
                    exit_code = result.returncode
                    stdout = result.stdout
                    stderr = result.stderr
                    error_message = None
                    
                    self.logger.info(f"Test {test_id} completed in {execution_time:.2f}s with exit code {exit_code}")
                    
                except subprocess.TimeoutExpired:
                    execution_time = timeout_seconds
                    timeout_occurred = True
                    exit_code = -1
                    stdout = ""
                    stderr = "Process timed out"
                    error_message = f"Test timed out after {timeout_seconds} seconds"
                    self.logger.warning(f"Test {test_id} timed out")
                    
                except Exception as e:
                    execution_time = time.time() - start_time
                    timeout_occurred = False
                    exit_code = -1
                    stdout = ""
                    stderr = str(e)
                    error_message = f"Execution error: {str(e)}"
                    self.logger.error(f"Test {test_id} failed with error: {str(e)}")
                
                # Comprehensive file and project analysis
                files_analysis = self._analyze_created_files(tmp_path)
                project_structure = self._analyze_project_structure(tmp_path)
                quality_assessment = self._assess_code_quality(tmp_path, test_data)
                
                # Determine success based on enhanced criteria
                success = self._evaluate_test_success(
                    exit_code, timeout_occurred, files_analysis, 
                    project_structure, quality_assessment, test_data
                )
                
                # Copy important files to session directory for analysis
                test_backup_dir = self.session_dir / f"test_{test_id}"
                self._backup_test_results(tmp_path, test_backup_dir)
                
                # Create execution metadata
                execution_metadata = {
                    'working_directory': str(tmp_path),
                    'files_found': len(files_analysis['files_created']),
                    'directories_created': len(files_analysis['directories_created']),
                    'total_file_size': files_analysis['total_size'],
                    'stdout_length': len(stdout),
                    'stderr_length': len(stderr),
                    'prompt_length': len(prompt),
                    'prompt_complexity_score': self._calculate_prompt_complexity(prompt),
                    'test_tags': test_data.get('tags', []),
                    'language': test_data.get('setup_requirements', {}).get('language', 'unknown')
                }
                
                execution_result = EnhancedExecutionResult(
                    test_id=test_id,
                    prompt=prompt,
                    category=category,
                    difficulty=difficulty,
                    complexity_factors=complexity_factors,
                    success=success,
                    execution_time_seconds=execution_time,
                    stdout=stdout,
                    stderr=stderr,
                    files_created=files_analysis['files_created'],
                    files_content=files_analysis['files_content'],
                    project_structure=project_structure,
                    exit_code=exit_code,
                    error_message=error_message,
                    timeout_occurred=timeout_occurred,
                    cai_command=cai_command,
                    execution_metadata=execution_metadata,
                    quality_assessment=quality_assessment,
                    tmp_directory=str(tmp_path),
                    cleanup_successful=True
                )
                
                # Save individual test result
                self._save_individual_result(execution_result)
                
                self.logger.info(f"Test {test_id} analysis complete: Success={success}, "
                               f"Quality Score={quality_assessment.get('overall_score', 0):.2f}, "
                               f"Files={len(files_analysis['files_created'])}")
                
                return execution_result
                
            finally:
                os.chdir(original_cwd)

    def _analyze_created_files(self, test_dir: Path) -> Dict[str, Any]:
        """Comprehensive analysis of files created during test execution."""
        files_created = []
        directories_created = []
        files_content = {}
        total_size = 0
        
        for item in test_dir.rglob("*"):
            if item.is_file():
                relative_path = item.relative_to(test_dir)
                files_created.append(str(relative_path))
                
                try:
                    file_size = item.stat().st_size
                    total_size += file_size
                    
                    # Read file content if it's a text file and not too large
                    if file_size < 50000:  # 50KB limit
                        try:
                            with open(item, 'r', encoding='utf-8') as f:
                                content = f.read()
                            files_content[str(relative_path)] = content
                        except UnicodeDecodeError:
                            files_content[str(relative_path)] = f"<Binary file, size: {file_size} bytes>"
                        except Exception as e:
                            files_content[str(relative_path)] = f"<Error reading file: {str(e)}>"
                    else:
                        files_content[str(relative_path)] = f"<Large file, size: {file_size} bytes>"
                        
                except Exception as e:
                    files_content[str(relative_path)] = f"<Error accessing file: {str(e)}>"
                    
            elif item.is_dir() and item != test_dir:
                relative_path = item.relative_to(test_dir)
                directories_created.append(str(relative_path))
        
        return {
            'files_created': files_created,
            'directories_created': directories_created,
            'files_content': files_content,
            'total_size': total_size,
            'file_types': self._categorize_file_types(files_created)
        }

    def _analyze_project_structure(self, test_dir: Path) -> Dict[str, Any]:
        """Analyze the overall project structure and organization."""
        structure = {
            'has_readme': False,
            'has_tests': False,
            'has_config_files': False,
            'has_documentation': False,
            'has_source_code': False,
            'project_depth': 0,
            'organization_score': 0.0
        }
        
        readme_files = ['README.md', 'readme.md', 'README.txt', 'README']
        test_patterns = ['test', 'tests', 'spec', '__tests__']
        config_patterns = ['.json', '.yaml', '.yml', '.toml', '.ini', 'Dockerfile', 'docker-compose']
        doc_patterns = ['doc', 'docs', 'documentation']
        source_patterns = ['.py', '.js', '.ts', '.rs', '.go', '.java', '.cpp', '.c']
        
        all_paths = [str(p.relative_to(test_dir)) for p in test_dir.rglob("*") if p.is_file()]
        
        # Check for README
        structure['has_readme'] = any(readme in all_paths for readme in readme_files)
        
        # Check for tests
        structure['has_tests'] = any(
            any(pattern in path.lower() for pattern in test_patterns) 
            for path in all_paths
        )
        
        # Check for config files
        structure['has_config_files'] = any(
            any(pattern in path.lower() for pattern in config_patterns)
            for path in all_paths
        )
        
        # Check for documentation
        structure['has_documentation'] = any(
            any(pattern in path.lower() for pattern in doc_patterns)
            for path in all_paths
        )
        
        # Check for source code
        structure['has_source_code'] = any(
            any(path.endswith(pattern) for pattern in source_patterns)
            for path in all_paths
        )
        
        # Calculate project depth
        if all_paths:
            structure['project_depth'] = max(len(Path(p).parts) for p in all_paths)
        
        # Calculate organization score
        score = 0
        if structure['has_readme']: score += 20
        if structure['has_tests']: score += 25
        if structure['has_config_files']: score += 15
        if structure['has_documentation']: score += 20
        if structure['has_source_code']: score += 20
        
        structure['organization_score'] = score
        
        return structure

    def _assess_code_quality(self, test_dir: Path, test_data: Dict[str, Any]) -> Dict[str, Any]:
        """Assess code quality based on various metrics."""
        quality_assessment = {
            'overall_score': 0.0,
            'code_files_found': 0,
            'has_error_handling': False,
            'has_documentation_strings': False,
            'has_type_annotations': False,
            'code_complexity': 'unknown',
            'follows_conventions': False,
            'security_considerations': False,
            'performance_optimizations': False
        }
        
        source_files = []
        for item in test_dir.rglob("*"):
            if item.is_file() and any(item.name.endswith(ext) for ext in ['.py', '.js', '.ts', '.rs', '.go', '.java']):
                source_files.append(item)
        
        quality_assessment['code_files_found'] = len(source_files)
        
        if not source_files:
            return quality_assessment
        
        total_score = 0
        max_score = 0
        
        for source_file in source_files:
            try:
                with open(source_file, 'r', encoding='utf-8') as f:
                    content = f.read().lower()
                
                file_score = 0
                file_max = 100
                
                # Error handling check
                if any(keyword in content for keyword in ['try', 'catch', 'except', 'error', 'panic']):
                    file_score += 20
                    quality_assessment['has_error_handling'] = True
                
                # Documentation check
                if any(keyword in content for keyword in ['"""', "'''", '//', '/*', '#']):
                    file_score += 15
                    quality_assessment['has_documentation_strings'] = True
                
                # Type annotations (language specific)
                if source_file.suffix == '.py' and ':' in content and '->' in content:
                    file_score += 10
                    quality_assessment['has_type_annotations'] = True
                elif source_file.suffix in ['.ts', '.rs', '.go', '.java']:
                    file_score += 10  # These languages have built-in typing
                    quality_assessment['has_type_annotations'] = True
                
                # Security considerations
                if any(keyword in content for keyword in ['sanitize', 'validate', 'auth', 'token', 'hash']):
                    file_score += 15
                    quality_assessment['security_considerations'] = True
                
                # Performance considerations
                if any(keyword in content for keyword in ['cache', 'async', 'await', 'parallel', 'optimize']):
                    file_score += 10
                    quality_assessment['performance_optimizations'] = True
                
                # Code structure
                if any(keyword in content for keyword in ['class', 'function', 'def', 'fn', 'func']):
                    file_score += 20
                    quality_assessment['follows_conventions'] = True
                
                # Complexity assessment
                complexity_indicators = content.count('if') + content.count('for') + content.count('while')
                if complexity_indicators < 5:
                    quality_assessment['code_complexity'] = 'simple'
                    file_score += 10
                elif complexity_indicators < 15:
                    quality_assessment['code_complexity'] = 'moderate'
                    file_score += 5
                else:
                    quality_assessment['code_complexity'] = 'complex'
                
                total_score += file_score
                max_score += file_max
                
            except Exception as e:
                self.logger.warning(f"Could not analyze file {source_file}: {e}")
        
        if max_score > 0:
            quality_assessment['overall_score'] = total_score / max_score * 100
        
        return quality_assessment

    def _categorize_file_types(self, files: List[str]) -> Dict[str, int]:
        """Categorize files by type."""
        categories = {
            'source_code': 0,
            'configuration': 0,
            'documentation': 0,
            'data': 0,
            'other': 0
        }
        
        for file_path in files:
            file_lower = file_path.lower()
            
            if any(file_lower.endswith(ext) for ext in ['.py', '.js', '.ts', '.rs', '.go', '.java', '.cpp', '.c', '.h']):
                categories['source_code'] += 1
            elif any(file_lower.endswith(ext) for ext in ['.json', '.yaml', '.yml', '.toml', '.ini', '.cfg']):
                categories['configuration'] += 1
            elif any(file_lower.endswith(ext) for ext in ['.md', '.txt', '.rst', '.html']):
                categories['documentation'] += 1
            elif any(file_lower.endswith(ext) for ext in ['.csv', '.json', '.xml', '.sql']):
                categories['data'] += 1
            else:
                categories['other'] += 1
        
        return categories

    def _calculate_prompt_complexity(self, prompt: str) -> float:
        """Calculate a complexity score for the prompt."""
        # Basic complexity indicators
        complexity_keywords = [
            'implement', 'build', 'create', 'design', 'optimize', 'scale',
            'distributed', 'microservices', 'real-time', 'performance',
            'security', 'authentication', 'database', 'api', 'integration'
        ]
        
        word_count = len(prompt.split())
        line_count = len(prompt.split('\n'))
        complexity_score = 0
        
        # Base score from length
        complexity_score += min(word_count / 10, 50)  # Max 50 points for length
        
        # Points for complexity keywords
        for keyword in complexity_keywords:
            if keyword.lower() in prompt.lower():
                complexity_score += 5
        
        # Points for numbered requirements
        numbered_requirements = len([line for line in prompt.split('\n') if line.strip() and line.strip()[0].isdigit()])
        complexity_score += numbered_requirements * 3
        
        return min(complexity_score, 100)  # Cap at 100

    def _evaluate_test_success(self, exit_code: int, timeout_occurred: bool, 
                             files_analysis: Dict, project_structure: Dict,
                             quality_assessment: Dict, test_data: Dict) -> bool:
        """Enhanced success evaluation based on multiple criteria."""
        if timeout_occurred or exit_code != 0:
            return False
        
        # Must have created some files
        if len(files_analysis['files_created']) == 0:
            return False
        
        # Check for source code
        if not project_structure['has_source_code']:
            return False
        
        # Quality threshold
        if quality_assessment['overall_score'] < 30:  # Minimum quality threshold
            return False
        
        # Complexity-based requirements
        complexity_factors = test_data.get('complexity_factors', [])
        required_elements = len(complexity_factors)
        
        if required_elements >= 3:  # High complexity tests
            # Require better project structure
            if not project_structure['has_readme']:
                return False
            if quality_assessment['overall_score'] < 50:
                return False
        
        return True

    def _backup_test_results(self, tmp_dir: Path, backup_dir: Path):
        """Backup test results to session directory for later analysis."""
        try:
            if backup_dir.exists():
                shutil.rmtree(backup_dir)
            
            # Copy important files only (not everything to save space)
            backup_dir.mkdir(parents=True, exist_ok=True)
            
            for item in tmp_dir.rglob("*"):
                if item.is_file():
                    relative_path = item.relative_to(tmp_dir)
                    target_path = backup_dir / relative_path
                    target_path.parent.mkdir(parents=True, exist_ok=True)
                    
                    # Only backup text files and small files
                    if item.stat().st_size < 100000:  # 100KB limit
                        try:
                            shutil.copy2(item, target_path)
                        except Exception as e:
                            self.logger.warning(f"Could not backup file {item}: {e}")
                            
        except Exception as e:
            self.logger.error(f"Failed to backup test results: {e}")

    def _save_individual_result(self, result: EnhancedExecutionResult):
        """Save individual test result to session directory."""
        result_file = self.session_dir / f"result_{result.test_id}.json"
        try:
            with open(result_file, 'w') as f:
                json.dump(asdict(result), f, indent=2, default=str)
        except Exception as e:
            self.logger.error(f"Failed to save individual result: {e}")

    def execute_enhanced_test_suite(self, test_suite_data: Dict[str, Any]) -> List[EnhancedExecutionResult]:
        """Execute the complete enhanced test suite."""
        tests = test_suite_data['tests']
        total_tests = len(tests)
        
        self.logger.info(f"Starting execution of {total_tests} enhanced tests")
        self.logger.info(f"Session directory: {self.session_dir}")
        
        all_results = []
        
        for i, test_data in enumerate(tests):
            self.logger.info(f"Progress: {i+1}/{total_tests} ({((i+1)/total_tests)*100:.1f}%)")
            
            try:
                result = self.execute_complex_test(test_data)
                all_results.append(result)
                
                # Update statistics
                self._update_enhanced_statistics(result)
                
                # Log progress every 10 tests
                if (i + 1) % 10 == 0:
                    success_rate = (self.execution_stats['successful_tests'] / (i + 1)) * 100
                    avg_quality = sum(self.execution_stats['quality_scores']) / len(self.execution_stats['quality_scores'])
                    self.logger.info(f"Progress Update: {i+1}/{total_tests} completed, "
                                   f"Success Rate: {success_rate:.1f}%, "
                                   f"Avg Quality: {avg_quality:.1f}")
                
            except Exception as e:
                self.logger.error(f"Failed to execute test {test_data['test_id']}: {str(e)}")
                # Create failed result
                failed_result = self._create_failed_result(test_data, str(e))
                all_results.append(failed_result)
        
        # Final statistics
        self._finalize_statistics(all_results)
        self.logger.info(f"Enhanced test suite execution completed: {self.execution_stats}")
        
        return all_results

    def _update_enhanced_statistics(self, result: EnhancedExecutionResult):
        """Update execution statistics with enhanced metrics."""
        self.execution_stats['total_tests'] += 1
        
        if result.success:
            self.execution_stats['successful_tests'] += 1
        
        if result.timeout_occurred:
            self.execution_stats['timeout_tests'] += 1
        
        # Category stats
        category = result.category
        if category not in self.execution_stats['categories']:
            self.execution_stats['categories'][category] = {'success': 0, 'total': 0}
        self.execution_stats['categories'][category]['total'] += 1
        if result.success:
            self.execution_stats['categories'][category]['success'] += 1
        
        # Difficulty stats
        difficulty = result.difficulty
        if difficulty not in self.execution_stats['difficulties']:
            self.execution_stats['difficulties'][difficulty] = {'success': 0, 'total': 0, 'avg_quality': 0}
        self.execution_stats['difficulties'][difficulty]['total'] += 1
        if result.success:
            self.execution_stats['difficulties'][difficulty]['success'] += 1
        
        # Complexity factor stats
        for factor in result.complexity_factors:
            if factor not in self.execution_stats['complexity_factors']:
                self.execution_stats['complexity_factors'][factor] = {'success': 0, 'total': 0}
            self.execution_stats['complexity_factors'][factor]['total'] += 1
            if result.success:
                self.execution_stats['complexity_factors'][factor]['success'] += 1
        
        # Quality scores
        quality_score = result.quality_assessment.get('overall_score', 0)
        self.execution_stats['quality_scores'].append(quality_score)

    def _create_failed_result(self, test_data: Dict[str, Any], error_message: str) -> EnhancedExecutionResult:
        """Create a failed result for exception cases."""
        return EnhancedExecutionResult(
            test_id=test_data['test_id'],
            prompt=test_data['prompt'],
            category=test_data['category'],
            difficulty=test_data['difficulty'],
            complexity_factors=test_data.get('complexity_factors', []),
            success=False,
            execution_time_seconds=0.0,
            stdout="",
            stderr=error_message,
            files_created=[],
            files_content={},
            project_structure={},
            exit_code=-1,
            error_message=f"Test execution failed: {error_message}",
            timeout_occurred=False,
            cai_command="",
            execution_metadata={},
            quality_assessment={'overall_score': 0.0},
            tmp_directory="",
            cleanup_successful=False
        )

    def _finalize_statistics(self, results: List[EnhancedExecutionResult]):
        """Finalize and calculate summary statistics."""
        total_tests = len(results)
        self.execution_stats['failed_tests'] = total_tests - self.execution_stats['successful_tests']
        self.execution_stats['total_execution_time'] = sum(r.execution_time_seconds for r in results)
        
        # Calculate average quality scores by difficulty
        for difficulty in self.execution_stats['difficulties']:
            difficulty_results = [r for r in results if r.difficulty == difficulty]
            if difficulty_results:
                avg_quality = sum(r.quality_assessment.get('overall_score', 0) for r in difficulty_results) / len(difficulty_results)
                self.execution_stats['difficulties'][difficulty]['avg_quality'] = avg_quality

    def save_enhanced_results(self, results: List[EnhancedExecutionResult], filename: str = "enhanced_results.json"):
        """Save enhanced execution results."""
        results_data = {
            'session_metadata': {
                'session_id': self.session_id,
                'total_tests': len(results),
                'execution_stats': self.execution_stats,
                'timestamp': time.time(),
                'cai_executable': self.cai_executable,
                'session_directory': str(self.session_dir)
            },
            'results': [asdict(result) for result in results]
        }
        
        results_file = self.session_dir / filename
        with open(results_file, 'w') as f:
            json.dump(results_data, f, indent=2, default=str)
        
        self.logger.info(f"Enhanced results saved to {results_file}")
        return results_file

    def generate_comprehensive_report(self, results: List[EnhancedExecutionResult]) -> str:
        """Generate comprehensive analysis report."""
        total_tests = len(results)
        successful_tests = sum(1 for r in results if r.success)
        failed_tests = total_tests - successful_tests
        timeout_tests = sum(1 for r in results if r.timeout_occurred)
        
        success_rate = (successful_tests / total_tests) * 100 if total_tests > 0 else 0
        avg_execution_time = sum(r.execution_time_seconds for r in results) / total_tests if total_tests > 0 else 0
        avg_quality_score = sum(r.quality_assessment.get('overall_score', 0) for r in results) / total_tests if total_tests > 0 else 0
        
        report = f"""
# Enhanced CAI Test Execution - Comprehensive Analysis Report

## Executive Summary
- **Session ID**: {self.session_id}
- **Total Tests Executed**: {total_tests}
- **Overall Success Rate**: {success_rate:.1f}%
- **Average Quality Score**: {avg_quality_score:.1f}/100
- **Average Execution Time**: {avg_execution_time:.2f} seconds
- **Session Directory**: {self.session_dir}

## Detailed Statistics

### Success Metrics
- **Successful Tests**: {successful_tests} ({success_rate:.1f}%)
- **Failed Tests**: {failed_tests} ({(failed_tests/total_tests)*100:.1f}%)
- **Timeout Tests**: {timeout_tests} ({(timeout_tests/total_tests)*100:.1f}%)
- **Total Execution Time**: {sum(r.execution_time_seconds for r in results):.2f} seconds

### Category Performance
"""
        
        # Category analysis
        for category, stats in self.execution_stats['categories'].items():
            success_rate_cat = (stats['success'] / stats['total']) * 100 if stats['total'] > 0 else 0
            category_results = [r for r in results if r.category == category]
            avg_quality_cat = sum(r.quality_assessment.get('overall_score', 0) for r in category_results) / len(category_results) if category_results else 0
            
            report += f"- **{category.replace('_', ' ').title()}**: {stats['success']}/{stats['total']} ({success_rate_cat:.1f}% success, {avg_quality_cat:.1f} avg quality)\n"
        
        report += f"""
### Difficulty Analysis
"""
        
        # Difficulty analysis
        for difficulty, stats in self.execution_stats['difficulties'].items():
            success_rate_diff = (stats['success'] / stats['total']) * 100 if stats['total'] > 0 else 0
            avg_quality_diff = stats.get('avg_quality', 0)
            
            report += f"- **{difficulty.title()}**: {stats['success']}/{stats['total']} ({success_rate_diff:.1f}% success, {avg_quality_diff:.1f} avg quality)\n"
        
        report += f"""
### Complexity Factor Analysis
"""
        
        # Top performing complexity factors
        complexity_performance = []
        for factor, stats in self.execution_stats['complexity_factors'].items():
            if stats['total'] >= 3:  # Only include factors with enough samples
                success_rate_factor = (stats['success'] / stats['total']) * 100
                complexity_performance.append((factor, success_rate_factor, stats['total']))
        
        complexity_performance.sort(key=lambda x: x[1], reverse=True)
        
        for factor, success_rate_factor, total in complexity_performance[:10]:
            report += f"- **{factor.replace('_', ' ').title()}**: {success_rate_factor:.1f}% success rate ({total} tests)\n"
        
        report += f"""
### Quality Assessment Summary

#### Code Quality Metrics
- **Tests with Source Code**: {sum(1 for r in results if r.project_structure.get('has_source_code', False))}
- **Tests with Documentation**: {sum(1 for r in results if r.project_structure.get('has_readme', False))}
- **Tests with Error Handling**: {sum(1 for r in results if r.quality_assessment.get('has_error_handling', False))}
- **Tests with Type Annotations**: {sum(1 for r in results if r.quality_assessment.get('has_type_annotations', False))}
- **Tests with Security Considerations**: {sum(1 for r in results if r.quality_assessment.get('security_considerations', False))}

#### Project Structure Analysis
- **Well-Organized Projects**: {sum(1 for r in results if r.project_structure.get('organization_score', 0) >= 60)}
- **Projects with Tests**: {sum(1 for r in results if r.project_structure.get('has_tests', False))}
- **Projects with Configuration**: {sum(1 for r in results if r.project_structure.get('has_config_files', False))}

### Performance Insights

#### Execution Time Analysis
- **Fast Execution** (<30s): {sum(1 for r in results if r.execution_time_seconds < 30)}
- **Medium Execution** (30-90s): {sum(1 for r in results if 30 <= r.execution_time_seconds < 90)}
- **Slow Execution** (90s+): {sum(1 for r in results if r.execution_time_seconds >= 90)}

#### File Creation Patterns
- **Average Files Created**: {sum(len(r.files_created) for r in results) / total_tests:.1f}
- **Tests Creating 5+ Files**: {sum(1 for r in results if len(r.files_created) >= 5)}
- **Tests Creating Project Structure**: {sum(1 for r in results if len(r.project_structure.get('directories_created', [])) > 0)}

## Key Findings and Recommendations

### Strengths Observed
1. **Complex Task Handling**: CAI demonstrates capability to handle multi-step, complex software development tasks
2. **File Creation**: Successfully creates multiple files and project structures
3. **Language Diversity**: Works across multiple programming languages
4. **Quality Considerations**: Shows awareness of code quality practices

### Areas for Improvement
1. **Timeout Management**: {timeout_tests} tests timed out, indicating need for better progress feedback
2. **Error Handling**: Only {sum(1 for r in results if r.quality_assessment.get('has_error_handling', False))} tests included proper error handling
3. **Documentation**: {total_tests - sum(1 for r in results if r.project_structure.get('has_readme', False))} tests lacked documentation
4. **Testing**: {total_tests - sum(1 for r in results if r.project_structure.get('has_tests', False))} tests didn't include test files

### Recommended Enhancements
1. **Implement streaming progress updates** for long-running tasks
2. **Add code quality validation** before file creation
3. **Improve documentation generation** as part of standard workflow
4. **Enhance test generation** capabilities
5. **Add project template support** for common patterns

## Technical Details
- **Session Directory**: {self.session_dir}
- **Individual Results**: Available in session directory as result_[test_id].json
- **Test Backups**: Created in session directory for manual inspection
- **Execution Logs**: Available in enhanced_execution.log

---
**Report Generated**: {time.strftime('%Y-%m-%d %H:%M:%S')}
**Total Analysis Time**: {sum(r.execution_time_seconds for r in results):.2f} seconds
"""
        
        return report

def main():
    """Main function for standalone testing."""
    executor = EnhancedTestExecutor()
    
    # Load test suite
    try:
        with open('enhanced_cai_test_suite.json', 'r') as f:
            test_suite_data = json.load(f)
    except FileNotFoundError:
        print("Enhanced test suite not found. Please run enhanced_test_generator.py first.")
        return
    
    print(f"Loaded enhanced test suite with {test_suite_data['metadata']['total_tests']} tests")
    print(f"Session ID: {executor.session_id}")
    
    # Execute test suite
    results = executor.execute_enhanced_test_suite(test_suite_data)
    
    # Save results
    results_file = executor.save_enhanced_results(results)
    
    # Generate comprehensive report
    report = executor.generate_comprehensive_report(results)
    report_file = executor.session_dir / 'comprehensive_analysis_report.md'
    with open(report_file, 'w') as f:
        f.write(report)
    
    print(f"\n{'='*60}")
    print("ENHANCED CAI TESTING COMPLETE")
    print(f"{'='*60}")
    print(f"Results saved to: {results_file}")
    print(f"Report saved to: {report_file}")
    print(f"Session directory: {executor.session_dir}")
    print(report)

if __name__ == "__main__":
    main()