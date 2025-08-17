#!/usr/bin/env python3
"""
Enhanced Pilot Test Runner - Execute a subset of complex tests to validate framework
and investigate CAI's capabilities with sophisticated prompts.
"""

import json
import sys
from enhanced_test_executor import EnhancedTestExecutor

def create_pilot_subset():
    """Create a subset of 10 representative complex tests."""
    
    # Load the full test suite
    with open('enhanced_cai_test_suite.json', 'r') as f:
        full_suite = json.load(f)
    
    # Select representative tests across categories and difficulties
    pilot_tests = []
    
    # Select tests to cover different categories and complexities
    categories_to_test = [
        'full_stack_development',
        'system_design', 
        'performance_optimization',
        'security_implementation',
        'data_processing',
        'api_development'
    ]
    
    # Find representative tests for each category
    for category in categories_to_test:
        category_tests = [t for t in full_suite['tests'] if t['category'] == category]
        if category_tests:
            # Pick the first test from each category
            pilot_tests.append(category_tests[0])
            if len(pilot_tests) >= 6:
                break
    
    # Add a few more tests of varying difficulty
    remaining_tests = [t for t in full_suite['tests'] if t not in pilot_tests]
    
    # Add one intermediate test
    intermediate_tests = [t for t in remaining_tests if t['difficulty'] == 'intermediate']
    if intermediate_tests:
        pilot_tests.append(intermediate_tests[0])
    
    # Add one expert test  
    expert_tests = [t for t in remaining_tests if t['difficulty'] == 'expert']
    if expert_tests:
        pilot_tests.append(expert_tests[0])
    
    # Fill to 10 tests
    while len(pilot_tests) < 10 and remaining_tests:
        next_test = remaining_tests.pop(0)
        if next_test not in pilot_tests:
            pilot_tests.append(next_test)
    
    # Create pilot suite structure
    pilot_suite = {
        "metadata": {
            "total_tests": len(pilot_tests),
            "categories": {},
            "difficulty_distribution": {},
            "description": "Enhanced pilot test suite with complex, real-world scenarios",
            "purpose": "Validate enhanced testing framework and investigate CAI capabilities"
        },
        "tests": pilot_tests
    }
    
    # Calculate metadata
    for test in pilot_tests:
        cat = test['category']
        pilot_suite['metadata']['categories'][cat] = pilot_suite['metadata']['categories'].get(cat, 0) + 1
        
        diff = test['difficulty'] 
        pilot_suite['metadata']['difficulty_distribution'][diff] = pilot_suite['metadata']['difficulty_distribution'].get(diff, 0) + 1
    
    # Save pilot suite
    with open('enhanced_pilot_suite.json', 'w') as f:
        json.dump(pilot_suite, f, indent=2)
    
    return pilot_suite

def main():
    """Run enhanced pilot test suite."""
    print("Enhanced CAI Pilot Test - Complex Scenario Evaluation")
    print("=" * 60)
    
    # Create pilot subset
    print("Creating pilot test subset...")
    pilot_suite = create_pilot_subset()
    
    print(f"Created pilot suite with {pilot_suite['metadata']['total_tests']} tests:")
    for category, count in pilot_suite['metadata']['categories'].items():
        print(f"  - {category}: {count} tests")
    
    print(f"Difficulty distribution: {pilot_suite['metadata']['difficulty_distribution']}")
    
    # Initialize enhanced executor
    print("\nInitializing enhanced test executor...")
    executor = EnhancedTestExecutor()
    
    print(f"Session ID: {executor.session_id}")
    print(f"Results will be stored in: {executor.session_dir}")
    
    # Execute pilot tests
    print("\nExecuting enhanced pilot tests...")
    print("Note: Each test may take 30-180 seconds depending on complexity")
    
    try:
        results = executor.execute_enhanced_test_suite(pilot_suite)
        
        # Save results
        results_file = executor.save_enhanced_results(results, "pilot_results.json")
        
        # Generate report
        report = executor.generate_comprehensive_report(results)
        report_file = executor.session_dir / 'pilot_analysis_report.md'
        with open(report_file, 'w') as f:
            f.write(report)
        
        # Print summary
        print("\n" + "=" * 60)
        print("ENHANCED PILOT TEST COMPLETE")
        print("=" * 60)
        
        successful_tests = sum(1 for r in results if r.success)
        success_rate = (successful_tests / len(results)) * 100
        avg_quality = sum(r.quality_assessment.get('overall_score', 0) for r in results) / len(results)
        
        print(f"Tests Executed: {len(results)}")
        print(f"Success Rate: {success_rate:.1f}%")
        print(f"Average Quality Score: {avg_quality:.1f}/100")
        print(f"Session Directory: {executor.session_dir}")
        print(f"Detailed Report: {report_file}")
        
        # Show interesting findings
        print("\n🔍 Key Findings:")
        
        # Files created analysis
        total_files = sum(len(r.files_created) for r in results)
        print(f"📁 Total files created: {total_files}")
        
        # Project structure analysis
        with_readme = sum(1 for r in results if r.project_structure.get('has_readme', False))
        with_tests = sum(1 for r in results if r.project_structure.get('has_tests', False))
        with_source = sum(1 for r in results if r.project_structure.get('has_source_code', False))
        
        print(f"📋 Projects with README: {with_readme}/{len(results)}")
        print(f"🧪 Projects with tests: {with_tests}/{len(results)}")
        print(f"💻 Projects with source code: {with_source}/{len(results)}")
        
        # Quality insights
        with_error_handling = sum(1 for r in results if r.quality_assessment.get('has_error_handling', False))
        with_security = sum(1 for r in results if r.quality_assessment.get('security_considerations', False))
        
        print(f"🛡️  Code with error handling: {with_error_handling}/{len(results)}")
        print(f"🔒 Code with security considerations: {with_security}/{len(results)}")
        
        # Performance insights
        timeouts = sum(1 for r in results if r.timeout_occurred)
        fast_execution = sum(1 for r in results if r.execution_time_seconds < 30)
        
        print(f"⏱️  Fast execution (<30s): {fast_execution}/{len(results)}")
        print(f"⏰ Timeouts: {timeouts}/{len(results)}")
        
        print(f"\n📊 Detailed analysis available in: {report_file}")
        
        return True
        
    except Exception as e:
        print(f"❌ Pilot test failed: {str(e)}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)