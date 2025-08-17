#!/usr/bin/env python3

import argparse

def main():
    parser = argparse.ArgumentParser(description='A new CLI application.')
    parser.add_argument('--version', action='version', version='%(prog)s 0.1.0')
    args = parser.parse_args()

    print('Hello from the new CLI app!')

if __name__ == '__main__':
    main()