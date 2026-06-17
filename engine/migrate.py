import os
import sys

def migrate_file(filepath, crate_name):
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    new_lines = []
    test_lines = []
    
    in_test = False
    brace_depth = 0
    test_buffer = []

    i = 0
    while i < len(lines):
        line = lines[i]
        
        if not in_test:
            if line.strip() == '#[cfg(test)]':
                in_test = True
                brace_depth = 0
                test_buffer = [line]
            else:
                new_lines.append(line)
        else:
            test_buffer.append(line)
            brace_depth += line.count('{')
            brace_depth -= line.count('}')
            
            # If we hit 0 braces and we've seen at least one brace, the block is done.
            # But wait, what if it's `#[cfg(test)]\n#[test]\nfn foo() {}`?
            # It starts with 0 braces.
            # So let's accumulate until brace_depth == 0 AND we've seen a `{`.
            if '{' in line:
                has_seen_brace = True
            
            if brace_depth == 0 and any('{' in l for l in test_buffer):
                in_test = False
                test_lines.extend(test_buffer)
                test_buffer = []
        i += 1

    # In case there's a dangling test_buffer
    if test_buffer:
        # It means we ended the file inside a test? That's weird but just append it
        test_lines.extend(test_buffer)

    if test_lines:
        with open(filepath, 'w') as f:
            f.write('\n'.join(new_lines))
        
        # generate module name from filepath
        mod_name = os.path.basename(filepath).replace('.rs', '')
        if mod_name == 'lib':
            mod_name = 'lib_tests'
            
        test_content = '\n'.join(test_lines)
        # basic super:: replacement, might need manual fixing
        test_content = test_content.replace('super::', f'specify_{crate_name}::')
        test_content = test_content.replace('crate::', f'specify_{crate_name}::')
        
        test_file = f'crates/{crate_name}/tests/integration.rs'
        with open(test_file, 'a') as f:
            f.write(f'\nmod {mod_name} {{\n')
            f.write(test_content)
            f.write('\n}\n')

if __name__ == '__main__':
    crate = sys.argv[1]
    crate_dir = f'crates/{crate}/src'
    for root, dirs, files in os.walk(crate_dir):
        for file in files:
            if file.endswith('.rs'):
                migrate_file(os.path.join(root, file), crate.replace('-', '_'))