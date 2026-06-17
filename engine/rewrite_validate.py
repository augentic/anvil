import re

with open('tests/slice/validate.rs', 'r') as f:
    content = f.read()

# We want to replace:
#     let assert = specify_cmd()
#         .current_dir(project.root())
#         .args(["--format", "json", "slice", "validate", "my-slice"])
#         .assert()
#         .failure();
#     assert_eq!(assert.get_output().status.code(), Some(2));
#     assert_provenance_fail_rule(assert.get_output(), "spec.requirement-id-missing");
# with:
#     let slice_dir = project.root().join(".specify/slices/my-slice");
#     let diagnostics = specify_model::validate::validate_slice(&slice_dir).expect("validate");
#     assert_provenance_fail_rule(&diagnostics, "spec.requirement-id-missing");

# Regex to match the specify_cmd() block and the assert_eq!
pattern = re.compile(r'let assert = specify_cmd\(\).*?\.failure\(\);\s*assert_eq!\(assert\.get_output\(\)\.status\.code\(\), Some\(2\)\);\s*assert_provenance_fail_rule\(assert\.get_output\(\), "([^"]+)"\);', re.DOTALL)

def repl(m):
    rule = m.group(1)
    return f'''let slice_dir = project.root().join(".specify/slices/my-slice");
    let diagnostics = specify_model::validate::validate_slice(&slice_dir).expect("validate");
    assert_provenance_fail_rule(&diagnostics, "{rule}");'''

new_content = pattern.sub(repl, content)

with open('tests/slice/validate.rs', 'w') as f:
    f.write(new_content)
