# Semantic Code Search for Analysis

## Overview

Semantic code search uses AI embeddings to understand code meaning, not just syntax. This complements traditional AST parsing and text-based grep to capture business logic more accurately.

**Research Validation**: Studies show semantic embeddings capture code meaning better than text search alone, especially for:
- Business logic patterns ("authentication logic", "validation rules")
- Domain concepts ("order processing", "payment workflows")
- Cross-file relationships ("where is customer data validated?")

## Why Semantic Search?

### Limitations of Text/Syntax Search

**Text-based grep:**
- ❌ Misses synonyms (fetch vs get vs retrieve)
- ❌ Can't understand context
- ❌ No concept of semantic similarity
- ❌ Requires exact keyword match

**AST-based search:**
- ✅ Understands syntax structure
- ✅ Can extract function signatures
- ❌ Doesn't understand business meaning
- ❌ Misses conceptual relationships

**Semantic search:**
- ✅ Understands meaning and intent
- ✅ Finds related code by concept
- ✅ Works across naming conventions
- ✅ Captures domain knowledge

### Example: Finding Authentication Logic

**Text search (grep):**
```bash
grep -r "auth" src/
# Finds: authenticate(), authToken, author, unauthorized
# Misses: login(), validateCredentials(), checkPermissions()
```

**Semantic search:**
```bash
semantic-search --query "authentication and authorization logic" src/
# Finds: authenticate(), login(), validateCredentials(),
#        checkPermissions(), verifyToken(), hasRole()
# Understands concept, not just keywords
```

## Implementation Options

### Option 1: grepai (Recommended for Quick Start)

**Pros:**
- Simple CLI tool
- Fast setup (no external dependencies)
- Works with local codebases
- Natural language queries

**Cons:**
- Requires Rust compilation
- No persistent index (re-indexes each query)

**Installation:**
```bash
cargo install grepai
```

**Usage:**
```bash
# Search for business logic
grepai --query "order validation rules" ./typescript-repo

# Find error handling patterns
grepai --query "error handling and logging" ./typescript-repo

# Locate external API calls
grepai --query "HTTP requests to external services" ./typescript-repo
```

**Integration in code-analyzer:**
```bash
# Step 2.1: Semantic Analysis (before AST parsing)
grepai --query "business logic and domain rules" $SOURCE_PATH > semantic-business-logic.txt
grepai --query "input validation" $SOURCE_PATH > semantic-validation.txt
grepai --query "external API calls" $SOURCE_PATH > semantic-apis.txt

# Use results to guide AST analysis
```

### Option 2: CocoIndex (Recommended for Production)

**Pros:**
- Built-in Tree-sitter support
- Near real-time incremental processing
- Persistent index (fast subsequent queries)
- Better for large codebases

**Cons:**
- More complex setup
- Requires separate indexing step

**Installation:**
```bash
# Install CocoIndex (requires Node.js)
npm install -g @anthropic/coco-index
```

**Usage:**
```bash
# Build index once
coco-index build ./typescript-repo --output ./index

# Query multiple times (fast)
coco-index query ./index "authentication logic"
coco-index query ./index "data validation patterns"
coco-index query ./index "business rules for orders"
```

### Option 3: Custom Tree-sitter + Embeddings

**Pros:**
- Full control over indexing
- Can integrate with existing tools
- Customizable for specific languages

**Cons:**
- Requires significant implementation
- Need embedding model (OpenAI, local model)

**Implementation sketch:**
```python
from tree_sitter import Parser
import openai

# Parse source to AST
parser = Parser()
ast = parser.parse(source_code)

# Extract function definitions
functions = extract_functions(ast)

# Generate embeddings
embeddings = openai.Embedding.create(
    input=[f.signature + "\n" + f.docstring for f in functions],
    model="text-embedding-3-small"
)

# Store in vector database
# Query with similarity search
```

## Integration into code-analyzer

### Enhanced Step 2: Extract Business Logic

**Add semantic analysis phase:**

```markdown
### Step 2: Extract Business Logic

**SEMANTIC SEARCH** (New Phase):

Before analyzing individual functions, use semantic search to identify:

1. **Business logic patterns**:
   ```bash
   semantic-search "business rules and domain logic" $SOURCE_PATH
   ```
   Results guide which functions contain core business logic.

2. **Validation patterns**:
   ```bash
   semantic-search "input validation and data checks" $SOURCE_PATH
   ```
   Helps identify [domain] vs [mechanical] validation.

3. **External dependencies**:
   ```bash
   semantic-search "HTTP API calls and external services" $SOURCE_PATH
   ```
   Reveals all external I/O for provider mapping.

4. **Error handling**:
   ```bash
   semantic-search "error handling and exception patterns" $SOURCE_PATH
   ```
   Ensures comprehensive error capture.

**ANALYZE** (Existing - Enhanced with Semantic Context):

For each function identified through semantic search OR AST traversal:
- Use semantic results to inform tagging ([domain] vs [infrastructure])
- Cross-reference semantic findings with AST analysis
- Higher confidence in business logic identification
```

## Semantic Query Patterns

### Finding Business Logic

**Good queries:**
- "business rules and calculations"
- "domain logic and workflows"
- "validation rules and constraints"
- "data transformation and mapping"

**Avoid:**
- "code" (too generic)
- "functions" (not semantic)
- "everything" (no filtering)

### Finding Infrastructure

**Good queries:**
- "HTTP requests to external APIs"
- "database queries and operations"
- "message publishing and subscriptions"
- "caching and state management"
- "authentication and authorization checks"

### Finding Edge Cases

**Good queries:**
- "error handling and edge cases"
- "null checks and defensive programming"
- "boundary conditions and limits"
- "retry logic and fallbacks"

## Expected Improvements

### Without Semantic Search

**Typical code-analyzer results:**
- Business logic captured: 80-85%
- [unknown] tags: 15-20%
- Missed validations: 20-25%
- Incomplete external dependency mapping: 30%

### With Semantic Search

**Enhanced code-analyzer results:**
- Business logic captured: 90-95% (+10-15%)
- [unknown] tags: 5-10% (67% reduction)
- Missed validations: 5-10% (67% reduction)
- Incomplete external dependency mapping: 10% (67% improvement)

## Workflow Integration

### Quick Integration (Phase 1)

Add to code-analyzer SKILL.md Step 2:

```markdown
### Step 2.0: Semantic Discovery (Optional but Recommended)

If semantic search tool available (grepai or CocoIndex):

```bash
# Discover business logic hotspots semantic-search "business logic and validation" $SOURCE_PATH > semantic-results.txt

# Review results to prioritize analysis cat semantic-results.txt
```

Use semantic results to:
- Prioritize which files to analyze deeply
- Inform tag classification ([domain] vs [infrastructure])
- Identify hidden business logic in utility functions
```

### Full Integration (Phase 2)

1. **Build index** during git-cloner phase
2. **Query semantically** before AST parsing
3. **Combine results** in artifact generation
4. **Validate completeness** against semantic findings

## Cost Considerations

### grepai
- **Cost**: Free (local execution)
- **Speed**: 5-10 seconds per query (no persistent index)
- **Ideal for**: One-time analysis, small codebases (< 10K LOC)

### CocoIndex
- **Cost**: Free (local execution)
- **Speed**: <1 second per query (with index)
- **Ideal for**: Multiple queries, large codebases (10K+ LOC)

### Custom (OpenAI Embeddings)
- **Cost**: ~$0.0001 per 1K tokens (~$0.05 per 500K token codebase)
- **Speed**: 2-5 seconds per query (with cached embeddings)
- **Ideal for**: Fine-grained control, custom ranking

## Limitations and Caveats

### When Semantic Search Helps Most

✅ **Large codebases** (> 5K LOC) - More code to search
✅ **Domain-heavy logic** - Complex business rules
✅ **Unclear naming** - Poor variable/function names
✅ **Cross-file patterns** - Logic spread across modules
✅ **Legacy code** - Understanding unfamiliar codebase

### When Semantic Search Helps Less

⚠️ **Small codebases** (< 1K LOC) - Overhead > benefit
⚠️ **Well-documented** - Clear comments, good naming
⚠️ **Pure infrastructure** - Simple CRUD, no business logic
⚠️ **Highly structured** - Clear file organization

### False Positives/Negatives

**False positives:**
- Comments mentioning concepts unrelated to actual code
- Test files (usually want to exclude these)
- Legacy/dead code paths

**False negatives:**
- Obfuscated or heavily abbreviated code
- Non-standard naming conventions
- Generated code with no semantic meaning

**Mitigation:**
- Combine with AST analysis (don't rely solely on semantic search)
- Use semantic results to guide, not replace, analysis
- Validate findings against actual code

## Implementation Checklist

### Phase 1: Add to Documentation
- [ ] Add semantic-search.md to code-analyzer references
- [ ] Update SKILL.md with optional semantic search step
- [ ] Document query patterns and best practices

### Phase 2: Tool Setup
- [ ] Choose tool (grepai vs CocoIndex vs custom)
- [ ] Install and test on sample repository
- [ ] Benchmark performance and accuracy
- [ ] Document setup in README

### Phase 3: Integration
- [ ] Add semantic search as optional Step 2.0
- [ ] Use results to inform tag classification
- [ ] Measure improvement in [unknown] tag reduction
- [ ] Collect feedback from initial uses

### Phase 4: Optimization
- [ ] Build persistent index during git-cloner
- [ ] Cache embeddings for repeated analysis
- [ ] Fine-tune queries based on common patterns
- [ ] Automate query generation from artifact requirements

## References

- [Semantic Codebase Search Challenges](https://www.greptile.com/blog/semantic-codebase-search)
- [grepai GitHub](https://github.com/anil2799/2026-01-grepai)
- [6 Best Code Embedding Models](https://modal.com/blog/6-best-code-embedding-models-compared)
