#!/usr/bin/env python3
"""
Test a single complex prompt to validate CAI functionality.
"""

import json
import tempfile
import subprocess
import os
from pathlib import Path

def test_single_complex_prompt():
    """Test CAI with a single complex prompt."""
    
    # Simple but multi-step prompt
    prompt = """Create a Python web API with the following features:
1. A Flask application with two endpoints: /health and /users
2. User data should be stored in a simple list
3. Include proper error handling
4. Add a requirements.txt file
5. Include a README.md with setup instructions"""
    
    print("Testing CAI with complex prompt...")
    print(f"Prompt: {prompt[:100]}...")
    
    with tempfile.TemporaryDirectory(prefix="cai_single_test_") as tmp_dir:
        tmp_path = Path(tmp_dir)
        print(f"Working directory: {tmp_path}")
        
        original_cwd = os.getcwd()
        
        try:
            os.chdir(tmp_path)
            
            # Create input file to avoid shell escaping issues
            input_file = tmp_path / "input.txt"
            with open(input_file, 'w') as f:
                f.write(prompt + "\nquit\n")
            
            # Execute CAI
            cai_command = f"cat {input_file} | {original_cwd}/target/release/cai chat"
            print(f"Executing: {cai_command}")
            
            result = subprocess.run(
                cai_command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=60,
                cwd=tmp_path
            )
            
            print(f"Exit code: {result.returncode}")
            print(f"STDOUT length: {len(result.stdout)}")
            print(f"STDERR length: {len(result.stderr)}")
            
            if result.stdout:
                print(f"STDOUT (first 500 chars):\n{result.stdout[:500]}")
            
            if result.stderr:
                print(f"STDERR (first 500 chars):\n{result.stderr[:500]}")
            
            # Check created files
            files_created = list(tmp_path.glob("*"))
            print(f"\nFiles created: {len(files_created)}")
            
            for file_path in files_created:
                if file_path.is_file() and file_path.name != "input.txt":
                    print(f"📄 {file_path.name} ({file_path.stat().st_size} bytes)")
                    
                    # Show content for small text files
                    if file_path.stat().st_size < 1000 and file_path.suffix in ['.py', '.txt', '.md']:
                        try:
                            with open(file_path, 'r') as f:
                                content = f.read()
                            print(f"   Content preview: {content[:200]}...")
                        except Exception as e:
                            print(f"   Could not read: {e}")
            
            success = (
                result.returncode == 0 and 
                len(files_created) > 1 and  # At least input.txt + created files
                any(f.name.endswith('.py') for f in files_created)
            )
            
            print(f"\nTest result: {'✅ SUCCESS' if success else '❌ FAILED'}")
            return success
            
        except subprocess.TimeoutExpired:
            print("❌ Test timed out")
            return False
        except Exception as e:
            print(f"❌ Test failed with error: {e}")
            return False
        finally:
            os.chdir(original_cwd)

if __name__ == "__main__":
    success = test_single_complex_prompt()
    exit(0 if success else 1)