#!/usr/bin/env python3
"""
Frontend i18n Checker
Scans frontend component directories for unlocalized CJK characters outside
comments and getLL() / LL calls.
"""

import os
import re
import sys

CJK_REGEX = re.compile(r'[\u4e00-\u9fff]')

DEFAULT_DIRS = [
    'src/components/canvas',
    'src/components/execution/viewers',
    'src/components/ui',
]

def check_file(filepath):
    violations = []
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()

    # Strip JSX comments {/* ... */}
    content = re.sub(r'\{\s*/\*[\s\S]*?\*/\s*\}', '', content)

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

        cleaned_line = ''.join(cleaned_chars).strip()
        if not cleaned_line:
            continue

        # Ignore lines calling localization helpers
        if 'getLL(' in cleaned_line or 'LL.' in cleaned_line:
            continue

        # Ignore console logging messages if present
        if cleaned_line.startswith('console.') or 'console.error(' in cleaned_line or 'console.warn(' in cleaned_line:
            continue

        if CJK_REGEX.search(cleaned_line):
            violations.append((line_num, cleaned_line))

    return violations

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
    target_dirs = sys.argv[1:] if len(sys.argv) > 1 else DEFAULT_DIRS

    total_violations = 0
    scanned_files = 0

    for rel_dir in target_dirs:
        abs_dir = os.path.join(repo_root, rel_dir)
        if not os.path.exists(abs_dir):
            continue
        for root, _, files in os.walk(abs_dir):
            for f in files:
                if f.endswith('.ts') or f.endswith('.tsx'):
                    scanned_files += 1
                    filepath = os.path.join(root, f)
                    rel_path = os.path.relpath(filepath, repo_root)
                    violations = check_file(filepath)
                    if violations:
                        for line_num, text in violations:
                            print(f"❌ {rel_path}:{line_num}: Hardcoded CJK found: {text}")
                            total_violations += 1

    print(f"\nScanned {scanned_files} files across {len(target_dirs)} directories.")
    if total_violations > 0:
        print(f"❌ Frontend i18n check failed: {total_violations} violation(s) found.")
        sys.exit(1)
    else:
        print("✅ Frontend i18n check passed: No hardcoded CJK text found in target component directories.")
        sys.exit(0)

if __name__ == '__main__':
    main()
