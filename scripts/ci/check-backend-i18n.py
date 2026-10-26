#!/usr/bin/env python3
"""
Backend i18n Checker
Scans Rust backend code (src-tauri/**/*.rs) for CJK characters in string literals
outside comments and unit test modules.
"""

import os
import re
import sys

CJK_REGEX = re.compile(r'[\u4e00-\u9fff]')

def check_file(filepath):
    violations = []
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()

    # Ignore unit test module portion (#[cfg(test)] mod tests)
    test_match = re.search(r'#\[cfg\(test\)\]\s*mod\s+tests', content)
    if test_match:
        content = content[:test_match.start()]

    lines = content.splitlines()
    in_block_comment = False

    for line_num, line in enumerate(lines, start=1):
        i = 0
        cleaned_chars = []
        while i < len(line):
            if in_block_comment:
                if line[i:i+2] == '*/':
                    in_block_comment = False
                    i += 2
                else:
                    i += 1
            else:
                if line[i:i+2] == '/*':
                    in_block_comment = True
                    i += 2
                elif line[i:i+2] == '//':
                    break
                else:
                    cleaned_chars.append(line[i])
                    i += 1

        cleaned_line = ''.join(cleaned_chars)
        # Scan string literals: regular "..." or raw r#"..."#
        # Non-greedy match within cleaned line
        matches = re.finditer(r'(?:r#*"(.*?)"#*|"(.*?)(?<!\\)")', cleaned_line)
        for m in matches:
            s = m.group(1) if m.group(1) is not None else m.group(2)
            if s and CJK_REGEX.search(s):
                violations.append((line_num, s.strip()))

    return violations

def main():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'src-tauri'))
    total_violations = 0

    for root, dirs, files in os.walk(root_dir):
        dirs[:] = [d for d in dirs if d not in ('target', 'tests', '.git')]
        for f in files:
            if f.endswith('.rs'):
                filepath = os.path.join(root, f)
                rel_path = os.path.relpath(filepath, os.path.join(root_dir, '..'))
                violations = check_file(filepath)
                if violations:
                    for line_num, text in violations:
                        print(f"❌ {rel_path}:{line_num}: String literal contains CJK: \"{text}\"")
                        total_violations += 1

    if total_violations > 0:
        print(f"\nTotal violations found: {total_violations}")
        sys.exit(1)
    else:
        print("✅ Backend i18n check passed: No CJK string literals found in production Rust code.")
        sys.exit(0)

if __name__ == '__main__':
    main()
