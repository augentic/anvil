import os

def remove_test_blocks(filepath):
    with open(filepath, 'r') as f:
        lines = f.readlines()

    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.strip() == '#[cfg(test)]':
            # look ahead to see what follows
            next_line = lines[i+1].strip() if i+1 < len(lines) else ""
            if next_line.startswith('mod ') and next_line.endswith(';'):
                # it's a module declaration like mod test_support;
                i += 2
                continue
            
            # otherwise it's a block
            in_test = True
            brace_depth = 0
            has_seen_brace = False
            
            # skip the #[cfg(test)]
            i += 1
            while i < len(lines):
                test_line = lines[i]
                brace_depth += test_line.count('{')
                brace_depth -= test_line.count('}')
                if '{' in test_line:
                    has_seen_brace = True
                
                i += 1
                if has_seen_brace and brace_depth == 0:
                    break
            continue
        else:
            new_lines.append(line)
            i += 1

    with open(filepath, 'w') as f:
        f.writelines(new_lines)

for root, dirs, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            remove_test_blocks(os.path.join(root, file))
