with open('branding/names.md', 'r') as f:
    content = f.read()

insert_rows = """| 5    | **Enantia**                    | org/platform | The opposing, balancing force (Greek *enantios*). If AI is the chaotic forward rush, Enantia is the structural counter-force that holds it in check. Clean mark; premium enterprise consultancy vibe. | ✅ no software mark; `.io`/`.ai`/`.dev` free; `.com` taken (Spanish CRO)                        |
| 6    | **Countertide**                | platform     | The direct pivot from Tidegate. The tide is the massive, unstoppable force of generative AI; the countertide is the opposing current that balances the flow, creating a navigable, governed channel.                             | ✅ no software mark; `.io`/`.ai`/`.dev` free; `.com` taken (GoDaddy 2002)                         |
"""

content = content.replace('| 7   | **Libratic', insert_rows + '| 7   | **Libratic')

with open('branding/names.md', 'w') as f:
    f.write(content)
