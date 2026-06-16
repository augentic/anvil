import re

with open('branding/names.md', 'r') as f:
    content = f.read()

# 1. Update the table
table_pattern = re.compile(r'(\| \d+\s+\| \*\*.*?\n)')
table_lines = []
for line in content.split('\n'):
    if line.startswith('| ') and '| Rank |' not in line and '| ---- |' not in line:
        match = re.match(r'\| (\d+)(\s+)\| (.*)', line)
        if match:
            rank = int(match.group(1))
            if rank >= 5:
                new_line = f"| {rank+2:<4}| {match.group(3)}"
                table_lines.append((line, new_line))

for old, new in table_lines:
    content = content.replace(old, new)

# Insert Enantia and Countertide into the table
insert_table_rows = """| 5    | **Enantia**                    | org/platform | The opposing, balancing force (Greek *enantios*). If AI is the chaotic forward rush, Enantia is the structural counter-force that holds it in check. Clean mark; premium enterprise consultancy vibe. | ✅ no software mark; `.io`/`.ai`/`.dev` free; `.com` taken (Spanish CRO)                        |
| 6    | **Countertide**                | platform     | The direct pivot from Tidegate. The tide is the massive, unstoppable force of generative AI; the countertide is the opposing current that balances the flow, creating a navigable, governed channel.                             | ✅ no software mark; `.io`/`.ai`/`.dev` free; `.com` taken (GoDaddy 2002)                         |
"""
content = content.replace('| 7    | **Libratic', insert_table_rows + '| 7    | **Libratic')

# 2. Update the headings
heading_lines = []
for line in content.split('\n'):
    match = re.match(r'^### (\d+)\. (.*)', line)
    if match:
        rank = int(match.group(1))
        if rank >= 5:
            new_line = f"### {rank+2}. {match.group(2)}"
            heading_lines.append((line, new_line))

heading_lines.sort(key=lambda x: int(re.match(r'^### (\d+)\.', x[0]).group(1)), reverse=True)

for old, new in heading_lines:
    content = content.replace(old, new)

# Insert Enantia and Countertide headings and details
new_details = """### 5. Enantia (en-AN-tee-uh) — org/platform

**The Metaphor:** From the Greek *enantios*, meaning opposite or against. The balancing, opposing force. If generative AI is the chaotic, infinite forward rush, *Enantia* is the structural counter-force that pushes back just enough to hold it in check and make it useful.
**The Pitch:** AI is the chaotic forward rush. *Enantia* is the governed return.
**The Vibe:** Premium enterprise consultancy. It carries the Latinate/Greek house style (*Omnia*, *Vectis*) and sounds like an established, 100-year-old advisory firm or control-systems contractor.
**Clearance:** ✅ **Verified clean of marks in software/AI (June 2026).** No `Enantia` trademark in any USPTO software class. The only major same-spelling use is a Spanish Contract Research Organization (CRO) in the pharma/biotech sector, which operates in a completely different class. **Domains:** ✅ `.io`, `.ai`, and `.dev` all unregistered and available; ❌ `.com` registered (held by the Spanish CRO).

### 6. Countertide — platform

**The Metaphor:** The direct pivot from Tidegate. The tide is the massive, unstoppable force of generative AI. The countertide is the opposing current that balances the flow, creating a navigable, governed channel. It keeps the physical infrastructure and fluid dynamics metaphor but focuses on the balancing force.
**The Pitch:** You can't stop the tide of generation. *Countertide* is the force that governs its flow.
**The Vibe:** Natural, powerful, physical infrastructure. It sounds highly engineered and deliberate, like a massive civil works project that tames a wild river.
**Clearance:** ✅ **Verified clean of marks in software/AI (June 2026).** No `Countertide` trademark in any USPTO software class. The term is primarily used as a vocabulary word in political discourse or as a character name in a video game (*Aether Gazer*). **Domains:** ✅ `.io`, `.ai`, and `.dev` all unregistered and available; ❌ `.com` registered (GoDaddy parked since 2002).

"""

content = content.replace('### 7. Libratic', new_details + '### 7. Libratic')

with open('branding/names.md', 'w') as f:
    f.write(content)

