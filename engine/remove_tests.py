import os
import sys
import re

def remove_test_blocks(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Find #[cfg(test)]
    # We want to remove the #[cfg(test)] attribute and the following item (module or function)
    
    lines = content.split('\n')
    new_lines = []
    
    in_test = False
    brace_depth = 0
    has_seen_brace = False

    i = 0
    while i < len(lines):
        line = lines[i]
        
        if not in_test:
            if line.strip() == '#[cfg(test)]':
                in_test = True
                brace_depth = 0
                has_seen_brace = False
            else:
                new_lines.append(line)
        else:
            brace_depth += line.count('{')
            brace_depth -= line.count('}')
            
            if '{' in line:
                has_seen_brace = True
            
            if has_seen_brace and brace_depth == 0:
                in_test = False
        i += 1

    with open(filepath, 'w') as f:
        f.write('\n'.join(new_lines))

if __name__ == '__main__':
    for d in ['diagnostics', 'extension-manifest', 'registry', 'vectis-shell-detect', 'schema', 'model', 'workflow', 'standards']:
        crate_dir = f'crates/{d}/src'
        if not os.path.exists(crate_dir):
            continue
        for root, dirs, files in os.walk(crate_dir):
            for file in files:
                if file.endswith('.rs'):
                    remove_test_blocks(os.path.join(root, file))
