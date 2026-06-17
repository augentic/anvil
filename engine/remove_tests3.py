import os
import sys

def remove_test_blocks(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    new_lines = []
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # If it's a #[cfg(test)] followed by #[test] or mod tests or fn 
        # let's be more specific to avoid false positives
        if line.strip().startswith('#[cfg(test)]'):
            # It's a test block
            # If it's a one-liner like #[cfg(test)] mod test_support;
            if i+1 < len(lines) and lines[i+1].strip().startswith('mod ') and lines[i+1].strip().endswith(';'):
                i += 2
                continue
            
            # Read characters to count braces, ignoring strings and comments
            # To do this safely, we will scan the remaining content of the file
            # from the start of the #[cfg(test)]
            
            # Re-construct remaining text
            remaining = '\n'.join(lines[i:])
            
            in_string = False
            in_comment = False
            in_block_comment = False
            brace_depth = 0
            has_seen_brace = False
            
            # Start after #[cfg(test)]
            idx = remaining.find('#[cfg(test)]') + len('#[cfg(test)]')
            
            while idx < len(remaining):
                c = remaining[idx]
                next_c = remaining[idx+1] if idx+1 < len(remaining) else ''
                
                if in_string:
                    if c == '\\':
                        idx += 2
                        continue
                    elif c == '"':
                        in_string = False
                elif in_comment:
                    if c == '\n':
                        in_comment = False
                elif in_block_comment:
                    if c == '*' and next_c == '/':
                        in_block_comment = False
                        idx += 2
                        continue
                else:
                    if c == '"':
                        in_string = True
                    elif c == '/' and next_c == '/':
                        in_comment = True
                    elif c == '/' and next_c == '*':
                        in_block_comment = True
                    elif c == '{':
                        brace_depth += 1
                        has_seen_brace = True
                    elif c == '}':
                        brace_depth -= 1
                        if has_seen_brace and brace_depth == 0:
                            # End of block!
                            idx += 1
                            break
                idx += 1
            
            if has_seen_brace and brace_depth == 0:
                # We found the end of the block
                # Skip lines that are fully inside this block
                block_text = remaining[:idx]
                block_lines = block_text.split('\n')
                i += len(block_lines) - 1
                
                # Check if there is anything left on the last line after the '}'
                leftover = remaining[idx:]
                # We will just continue to the next line in the outer loop
                # because `i` is now at the line where '}' was found
                # But wait, what if there's code after '}' on the same line?
                # Usually there isn't. So we just skip this line entirely.
                i += 1
                continue
            else:
                # Something went wrong, couldn't find matching brace
                # Just append the line and move on
                new_lines.append(line)
                i += 1
        else:
            new_lines.append(line)
            i += 1

    with open(filepath, 'w') as f:
        f.write('\n'.join(new_lines))

if __name__ == '__main__':
    for root, dirs, files in os.walk('crates'):
        for file in files:
            if file.endswith('.rs'):
                remove_test_blocks(os.path.join(root, file))
