import unittest
import subprocess
import sys

class TestCliApp(unittest.TestCase):
    def test_hello_output(self):
        result = subprocess.run([sys.executable, 'new_cli_app/src/main.py'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)
        self.assertIn('Hello from the new CLI app!', result.stdout)

if __name__ == '__main__':
    unittest.main()